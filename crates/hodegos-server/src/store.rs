//! Données de l'utilisateur pour un projet, sous `$XDG_DATA_HOME/hodegos/<id-projet>/` :
//! historique des remarques. Rien n'est jamais écrit dans le dépôt.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::protocol::Remark;

#[derive(Serialize, Deserialize, Default)]
struct RemarksFile {
    version: u32,
    remarks: Vec<Remark>,
}

pub(crate) struct Store {
    /// `None` : rien n'est persisté (dossier de données introuvable).
    dir: Option<PathBuf>,
    remarks: Mutex<Vec<Remark>>,
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
        if let Some(dir) = &dir {
            let project = serde_json::json!({ "root": root.display().to_string() });
            if let Err(err) = write_json(&dir.join("project.json"), &project) {
                tracing::warn!(dir = %dir.display(), error = %err, "dossier de données non écrit");
            }
        }
        Self {
            dir,
            remarks: Mutex::new(remarks),
        }
    }

    /// Remarques ouvertes d'un fichier.
    pub(crate) fn open_remarks(&self, path: &str) -> Vec<Remark> {
        self.remarks
            .lock()
            .map(|remarks| {
                remarks
                    .iter()
                    .filter(|r| r.path == path && r.status == "open")
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn add(&self, items: &[Remark]) {
        let Ok(mut remarks) = self.remarks.lock() else {
            return;
        };
        remarks.extend(items.iter().cloned());
        let Some(dir) = &self.dir else {
            return;
        };
        let file = RemarksFile {
            version: 1,
            remarks: remarks.clone(),
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
