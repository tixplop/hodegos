//! Un terminal : pseudo-terminal, processus lancé dedans, sorties regroupées et
//! contre-pression. Ce qui passe dans le terminal n'est jamais journalisé (tailles
//! seulement, au niveau `trace`).

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::time::Duration;

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::sync::{mpsc, oneshot};

/// Une sortie part au plus tard après ce délai…
const BATCH_DELAY: Duration = Duration::from_millis(16);
/// … ou dès qu'elle atteint cette taille.
pub(crate) const BATCH_MAX: usize = 64 * 1024;
const READ_BUFFER: usize = 16 * 1024;
/// Lectures en attente avant que la lecture du pseudo-terminal ne soit suspendue.
const READ_QUEUE: usize = 8;
/// Saisies en attente d'écriture dans le pseudo-terminal.
const INPUT_QUEUE: usize = 32;

/// Processus à lancer dans le pseudo-terminal.
pub(crate) struct Launch {
    pub(crate) program: PathBuf,
    pub(crate) args: Vec<String>,
    pub(crate) env: Vec<(String, String)>,
    pub(crate) cwd: PathBuf,
}

/// Ce que la connexion reçoit d'un terminal.
#[derive(Debug)]
pub(crate) enum Event {
    Output {
        id: u32,
        data: Vec<u8>,
    },
    /// Fin du processus ; `code` absent s'il a été tué par un signal.
    Exited {
        id: u32,
        code: Option<i32>,
    },
}

/// Terminal ouvert ; le fermer (ou le laisser tomber) tue son processus.
pub(crate) struct Session {
    master: Box<dyn MasterPty + Send>,
    input: SyncSender<Vec<u8>>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// Lance `launch` dans un pseudo-terminal ; ses sorties arrivent sur `events`, par lots
/// de [`BATCH_MAX`] octets au plus, toutes les 16 ms au plus. Si `events` est plein, la
/// lecture s'arrête et le noyau suspend le processus qui écrit.
pub(crate) fn spawn(
    launch: Launch,
    id: u32,
    cols: u16,
    rows: u16,
    events: mpsc::Sender<Event>,
) -> Result<Session, String> {
    let pair = native_pty_system()
        .openpty(size(cols, rows))
        .map_err(|e| format!("pseudo-terminal indisponible : {e}"))?;
    let mut command = CommandBuilder::new(&launch.program);
    command.args(&launch.args);
    command.env_clear();
    for (name, value) in &launch.env {
        command.env(name, value);
    }
    command.cwd(&launch.cwd);
    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|e| format!("le terminal ne démarre pas : {e}"))?;
    drop(pair.slave);
    let killer = child.clone_killer();
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("pseudo-terminal illisible : {e}"))?;
    let mut writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("pseudo-terminal non inscriptible : {e}"))?;

    let (raw_tx, mut raw_rx) = mpsc::channel::<Vec<u8>>(READ_QUEUE);
    std::thread::spawn(move || {
        let mut buffer = vec![0u8; READ_BUFFER];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if raw_tx.blocking_send(buffer[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let (exit_tx, exit_rx) = oneshot::channel();
    std::thread::spawn(move || {
        let code = child.wait().ok().and_then(|status| {
            if status.signal().is_some() {
                None
            } else {
                i32::try_from(status.exit_code()).ok()
            }
        });
        let _ = exit_tx.send(code);
    });

    let (input, input_rx) = sync_channel::<Vec<u8>>(INPUT_QUEUE);
    std::thread::spawn(move || {
        while let Ok(data) = input_rx.recv() {
            if writer
                .write_all(&data)
                .and_then(|()| writer.flush())
                .is_err()
            {
                break;
            }
        }
    });

    tokio::spawn(async move {
        let mut batch: Vec<u8> = Vec::new();
        let mut tick = tokio::time::interval(BATCH_DELAY);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                chunk = raw_rx.recv() => match chunk {
                    Some(chunk) => {
                        batch.extend(chunk);
                        while batch.len() >= BATCH_MAX {
                            let rest = batch.split_off(BATCH_MAX);
                            let data = std::mem::replace(&mut batch, rest);
                            if !send(&events, id, data).await {
                                return;
                            }
                        }
                    }
                    None => break,
                },
                _ = tick.tick() => {
                    if !batch.is_empty() && !send(&events, id, std::mem::take(&mut batch)).await {
                        return;
                    }
                }
            }
        }
        if !batch.is_empty() && !send(&events, id, batch).await {
            return;
        }
        let code = exit_rx.await.ok().flatten();
        let _ = events.send(Event::Exited { id, code }).await;
    });

    Ok(Session {
        master: pair.master,
        input,
        killer,
    })
}

async fn send(events: &mpsc::Sender<Event>, id: u32, data: Vec<u8>) -> bool {
    tracing::trace!(terminal = id, octets = data.len(), "sortie du terminal");
    events.send(Event::Output { id, data }).await.is_ok()
}

impl Session {
    /// Saisie ; refusée si le terminal ne lit plus (file pleine) ou s'il est fermé.
    pub(crate) fn write(&self, data: Vec<u8>) -> Result<(), &'static str> {
        match self.input.try_send(data) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err("le terminal ne lit plus sa saisie"),
            Err(TrySendError::Disconnected(_)) => Err("le terminal est fermé"),
        }
    }

    pub(crate) fn resize(&self, cols: u16, rows: u16) {
        if let Err(err) = self.master.resize(size(cols, rows)) {
            tracing::debug!(error = %err, "terminal non redimensionné");
        }
    }

    /// Tue le processus : `bwrap`, et avec lui tout l'espace de noms (`--unshare-pid`).
    pub(crate) fn kill(&mut self) {
        let _ = self.killer.kill();
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sans `bwrap` : un `sh` dans le pseudo-terminal, pour le regroupement et l'ordre.
    fn launch(script: &str) -> Launch {
        Launch {
            program: PathBuf::from("/bin/sh"),
            args: vec!["-c".to_owned(), script.to_owned()],
            env: vec![("PATH".to_owned(), "/usr/bin:/bin".to_owned())],
            cwd: std::env::temp_dir(),
        }
    }

    async fn collect(mut events: mpsc::Receiver<Event>) -> (Vec<Vec<u8>>, Option<i32>) {
        let mut outputs = Vec::new();
        loop {
            match tokio::time::timeout(Duration::from_secs(10), events.recv()).await {
                Ok(Some(Event::Output { data, .. })) => outputs.push(data),
                Ok(Some(Event::Exited { code, .. })) => return (outputs, code),
                other => panic!("fin inattendue : {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn les_sorties_arrivent_par_lots_bornes_puis_la_fin() {
        let (tx, rx) = mpsc::channel(1);
        // 300 000 octets, sans conversion de fin de ligne par le pseudo-terminal.
        let _session = spawn(
            launch("head -c 300000 /dev/zero | tr '\\0' x; exit 3"),
            7,
            80,
            24,
            tx,
        )
        .unwrap();
        let (outputs, code) = collect(rx).await;
        assert!(outputs.iter().all(|o| o.len() <= BATCH_MAX));
        assert_eq!(outputs.iter().map(Vec::len).sum::<usize>(), 300_000);
        assert_eq!(code, Some(3));
    }

    #[tokio::test]
    async fn un_lecteur_lent_suspend_le_processus_sans_rien_perdre() {
        let (tx, mut rx) = mpsc::channel(1);
        let _session = spawn(
            launch("head -c 2000000 /dev/zero | tr '\\0' y"),
            1,
            80,
            24,
            tx,
        )
        .unwrap();
        // Rien n'est lu pendant 300 ms : la lecture s'arrête, rien ne s'accumule sans borne.
        tokio::time::sleep(Duration::from_millis(300)).await;
        let mut total = 0;
        loop {
            match tokio::time::timeout(Duration::from_secs(10), rx.recv()).await {
                Ok(Some(Event::Output { data, .. })) => total += data.len(),
                Ok(Some(Event::Exited { .. })) => break,
                other => panic!("fin inattendue : {other:?}"),
            }
        }
        assert_eq!(total, 2_000_000);
    }

    #[tokio::test]
    async fn la_saisie_arrive_et_fermer_tue_le_processus() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut session = spawn(
            launch("read ligne; echo \"lu:$ligne\"; sleep 30"),
            2,
            80,
            24,
            tx,
        )
        .unwrap();
        session.write(b"bonjour\n".to_vec()).unwrap();
        let mut seen = Vec::new();
        while !String::from_utf8_lossy(&seen).contains("lu:bonjour") {
            match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
                Ok(Some(Event::Output { data, .. })) => seen.extend(data),
                other => panic!("pas de réponse : {other:?}"),
            }
        }
        session.kill();
        let code = loop {
            match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
                Ok(Some(Event::Exited { code, .. })) => break code,
                Ok(Some(Event::Output { .. })) => {}
                other => panic!("pas de fin : {other:?}"),
            }
        };
        assert_eq!(code, None, "tué par un signal");
    }
}
