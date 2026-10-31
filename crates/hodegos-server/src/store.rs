//! Données de l'utilisateur pour un projet, sous `$XDG_DATA_HOME/hodegos/<id-projet>/` :
//! historique des remarques et fichiers déjà vus par une passe d'ouverture. Rien
//! n'est jamais écrit dans le dépôt.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::protocol::{Anchor, Remark};

#[derive(Serialize, Deserialize, Default)]
struct RemarksFile {
    version: u32,
    remarks: Vec<Remark>,
}

#[derive(Serialize, Deserialize, Default)]
struct OpenedFile {
    version: u32,
    paths: BTreeSet<String>,
}

/// État des remarques et des passes d'ouverture, pour revenir en arrière.
pub(crate) struct Snapshot {
    remarks: Vec<Remark>,
    opened: BTreeSet<String>,
}

pub(crate) struct Store {
    /// `None` : rien n'est persisté (dossier de données introuvable).
    dir: Option<PathBuf>,
    remarks: Mutex<Vec<Remark>>,
    /// Fichiers dont la passe d'ouverture s'est terminée (`opened.json`).
    opened: Mutex<BTreeSet<String>>,
}

impl Store {
    /// Lit l'historique du projet ; note la racine du projet dans `project.json` pour
    /// qu'un humain s'y retrouve.
    pub(crate) fn open(dir: Option<PathBuf>, root: &Path) -> Self {
        let remarks = dir
            .as_ref()
            .and_then(|dir| fs::read_to_string(dir.join("remarks.json")).ok())
            .and_then(|text| match serde_json::from_str::<RemarksFile>(&text) {
                Ok(file) => Some(file.remarks),
                Err(err) => {
                    tracing::warn!(error = %err, "remarks.json illisible, historique ignoré");
                    None
                }
            })
            .unwrap_or_default();
        let opened = dir
            .as_ref()
            .and_then(|dir| fs::read_to_string(dir.join("opened.json")).ok())
            .and_then(|text| match serde_json::from_str::<OpenedFile>(&text) {
                Ok(file) => Some(file.paths),
                Err(err) => {
                    tracing::warn!(error = %err, "opened.json illisible, ignoré");
                    None
                }
            })
            .unwrap_or_default();
        if let Some(dir) = &dir {
            let project = serde_json::json!({ "root": root.display().to_string() });
            if let Err(err) = write_json(&dir.join("project.json"), &project) {
                tracing::warn!(dir = %dir.display(), error = %err, "dossier de données non écrit");
            }
        }
        Self {
            dir,
            remarks: Mutex::new(remarks),
            opened: Mutex::new(opened),
        }
    }

    /// Une remarque, quel que soit son statut, a déjà été faite sur ce fichier.
    pub(crate) fn has_remarks(&self, path: &str) -> bool {
        self.remarks
            .lock()
            .map(|remarks| remarks.iter().any(|r| r.path == path))
            .unwrap_or(true)
    }

    /// La passe d'ouverture de ce fichier a déjà abouti.
    pub(crate) fn was_opened(&self, path: &str) -> bool {
        self.opened
            .lock()
            .map(|opened| opened.contains(path))
            .unwrap_or(true)
    }

    pub(crate) fn mark_opened(&self, path: &str) {
        let Ok(mut opened) = self.opened.lock() else {
            return;
        };
        if opened.insert(path.to_owned()) {
            self.persist_opened(&opened);
        }
    }

    /// Oublie des fichiers supprimés depuis Hodegos (ADR 0006) : leurs remarques,
    /// quel que soit leur statut, et leur passe d'ouverture.
    pub(crate) fn forget(&self, paths: &HashSet<String>) {
        if let Ok(mut remarks) = self.remarks.lock() {
            let before = remarks.len();
            remarks.retain(|r| !paths.contains(&r.path));
            if remarks.len() != before {
                self.persist(&remarks);
            }
        }
        if let Ok(mut opened) = self.opened.lock() {
            let before = opened.len();
            opened.retain(|path| !paths.contains(path));
            if opened.len() != before {
                self.persist_opened(&opened);
            }
        }
    }

    /// Fait suivre leurs remarques et leur passe d'ouverture à des fichiers déplacés
    /// depuis Hodegos (ADR 0008), `moves` allant de l'ancien chemin au nouveau. Tout ou
    /// rien : si un fichier de données ne peut pas être écrit, rien ne change.
    pub(crate) fn move_paths(&self, moves: &HashMap<String, String>) -> std::io::Result<()> {
        let poisoned = || std::io::Error::other("verrou empoisonné");
        let mut remarks = self.remarks.lock().map_err(|_| poisoned())?;
        let mut opened = self.opened.lock().map_err(|_| poisoned())?;
        let mut moved_remarks = remarks.clone();
        let mut remarks_changed = false;
        for remark in &mut moved_remarks {
            if let Some(to) = moves.get(&remark.path) {
                remark.path.clone_from(to);
                remarks_changed = true;
            }
        }
        let moved_opened: BTreeSet<String> = opened
            .iter()
            .map(|path| moves.get(path).unwrap_or(path).clone())
            .collect();
        let opened_changed = opened.iter().any(|path| moves.contains_key(path));
        if remarks_changed {
            self.write_remarks(&moved_remarks)?;
        }
        if opened_changed {
            if let Err(err) = self.write_opened(&moved_opened) {
                if remarks_changed {
                    self.persist(&remarks);
                }
                return Err(err);
            }
        }
        *remarks = moved_remarks;
        *opened = moved_opened;
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> Option<Snapshot> {
        Some(Snapshot {
            remarks: self.remarks.lock().ok()?.clone(),
            opened: self.opened.lock().ok()?.clone(),
        })
    }

    /// Revient à un état enregistré par [`Self::snapshot`], en mémoire et sur le disque.
    pub(crate) fn restore(&self, snapshot: Snapshot) {
        if let Ok(mut remarks) = self.remarks.lock() {
            self.persist(&snapshot.remarks);
            *remarks = snapshot.remarks;
        }
        if let Ok(mut opened) = self.opened.lock() {
            self.persist_opened(&snapshot.opened);
            *opened = snapshot.opened;
        }
    }

    fn persist_opened(&self, opened: &BTreeSet<String>) {
        if let Err(err) = self.write_opened(opened) {
            tracing::error!(error = %err, "opened.json non écrit");
        }
    }

    fn write_opened(&self, opened: &BTreeSet<String>) -> std::io::Result<()> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        let file = OpenedFile {
            version: 1,
            paths: opened.clone(),
        };
        write_json(&dir.join("opened.json"), &file)
    }

    /// Remarques d'un fichier affichées dans le panneau : ouvertes, répondues ou détachées.
    pub(crate) fn visible_remarks(&self, path: &str) -> Vec<Remark> {
        self.remarks
            .lock()
            .map(|remarks| {
                remarks
                    .iter()
                    .filter(|r| r.path == path && r.is_visible())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn get(&self, id: &str) -> Option<Remark> {
        self.remarks
            .lock()
            .ok()
            .and_then(|remarks| remarks.iter().find(|r| r.id == id).cloned())
    }

    pub(crate) fn add(&self, items: &[Remark]) {
        let Ok(mut remarks) = self.remarks.lock() else {
            return;
        };
        remarks.extend(items.iter().cloned());
        self.persist(&remarks);
    }

    /// Modifie une remarque et l'enregistre ; renvoie son nouvel état, `None` si elle
    /// est introuvable.
    pub(crate) fn update(&self, id: &str, change: impl FnOnce(&mut Remark)) -> Option<Remark> {
        let mut remarks = self.remarks.lock().ok()?;
        let remark = remarks.iter_mut().find(|r| r.id == id)?;
        change(remark);
        let updated = remark.clone();
        self.persist(&remarks);
        Some(updated)
    }

    /// Recale les remarques actives d'un fichier : `locate` donne la ligne de l'ancre,
    /// ou `None` si la section ou la citation n'existe plus. Une remarque sans ancre
    /// passe en `detached`, définitivement ; renvoie celles-là.
    pub(crate) fn reanchor(
        &self,
        path: &str,
        locate: impl Fn(&Anchor) -> Option<usize>,
    ) -> Vec<Remark> {
        let Ok(mut remarks) = self.remarks.lock() else {
            return Vec::new();
        };
        let mut changed = false;
        let mut detached = Vec::new();
        for remark in remarks
            .iter_mut()
            .filter(|r| r.path == path && r.is_active())
        {
            match locate(&remark.anchor) {
                Some(line) if line == remark.anchor.line_hint => {}
                Some(line) => {
                    remark.anchor.line_hint = line;
                    changed = true;
                }
                None => {
                    remark.status = "detached".to_owned();
                    detached.push(remark.clone());
                    changed = true;
                }
            }
        }
        if changed {
            self.persist(&remarks);
        }
        detached
    }

    fn persist(&self, remarks: &[Remark]) {
        if let Err(err) = self.write_remarks(remarks) {
            tracing::error!(error = %err, "historique des remarques non écrit");
        }
    }

    fn write_remarks(&self, remarks: &[Remark]) -> std::io::Result<()> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        let file = RemarksFile {
            version: 1,
            remarks: remarks.to_vec(),
        };
        write_json(&dir.join("remarks.json"), &file)
    }
}

/// Écrit `value` en JSON par fichier temporaire renommé.
pub(crate) fn write_json(path: &Path, value: &impl Serialize) -> std::io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| std::io::Error::other("chemin sans dossier"))?;
    fs::create_dir_all(dir)?;
    let text = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    temp.write_all(text.as_bytes())?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}
