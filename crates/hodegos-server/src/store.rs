//! Données de l'utilisateur pour un projet, sous `$XDG_DATA_HOME/hodegos/<id-projet>/` :
//! historique des remarques et fichiers déjà vus par une passe d'ouverture. Rien
//! n'est jamais écrit dans le dépôt.

use std::collections::BTreeSet;
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
        if !opened.insert(path.to_owned()) {
            return;
        }
        let Some(dir) = &self.dir else {
            return;
        };
        let file = OpenedFile {
            version: 1,
            paths: opened.clone(),
        };
        if let Err(err) = write_json(&dir.join("opened.json"), &file) {
            tracing::error!(error = %err, "opened.json non écrit");
        }
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
        let Some(dir) = &self.dir else {
            return;
        };
        let file = RemarksFile {
            version: 1,
            remarks: remarks.to_vec(),
        };
        if let Err(err) = write_json(&dir.join("remarks.json"), &file) {
            tracing::error!(error = %err, "historique des remarques non écrit");
        }
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
