//! Instantanés des revues (phase 9c, docs/phase9.md, précision 2) : textes masqués
//! relus, rangés par empreinte sous `<id-projet>/snapshots/`, partagés entre mentors.
//! Le diff d'une revue part de l'instantané de la revue précédente du mentor ; aucun
//! secret n'y est copié, et un contenu qu'aucun manifeste ne cite plus est supprimé.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub(crate) fn digest(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .take(16)
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) struct Snapshots {
    /// `None` : pas de dossier de données, pas de diff entre revues.
    dir: Option<PathBuf>,
}

impl Snapshots {
    pub(crate) fn new(project_dir: Option<&Path>) -> Self {
        Self {
            dir: project_dir.map(|d| d.join("snapshots")),
        }
    }

    /// Range les textes (déjà masqués) et renvoie le manifeste chemin → empreinte.
    pub(crate) fn store(&self, texts: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        let mut manifest = BTreeMap::new();
        for (path, text) in texts {
            let hash = digest(text);
            if let Some(dir) = &self.dir {
                let file = dir.join(&hash);
                if !file.exists() {
                    let written = fs::create_dir_all(dir).and_then(|()| {
                        let mut temp = tempfile::NamedTempFile::new_in(dir)?;
                        std::io::Write::write_all(&mut temp, text.as_bytes())?;
                        temp.persist(&file).map(|_| ()).map_err(|e| e.error)
                    });
                    if let Err(err) = written {
                        tracing::warn!(error = %err, "instantané non écrit");
                    }
                }
            }
            manifest.insert(path.clone(), hash);
        }
        manifest
    }

    /// Textes d'un manifeste ; un contenu disparu compte comme un fichier vide.
    pub(crate) fn load(&self, manifest: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        manifest
            .iter()
            .map(|(path, hash)| {
                let text = self
                    .dir
                    .as_ref()
                    .and_then(|dir| fs::read_to_string(dir.join(hash)).ok())
                    .unwrap_or_default();
                (path.clone(), text)
            })
            .collect()
    }

    /// Supprime les contenus qu'aucun manifeste ne cite plus.
    pub(crate) fn collect(&self, keep: &BTreeSet<String>) {
        let Some(entries) = self.dir.as_ref().and_then(|dir| fs::read_dir(dir).ok()) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.len() == 32 && !keep.contains(&name) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_relit_et_fait_le_menage() {
        let dir = tempfile::tempdir().unwrap();
        let snapshots = Snapshots::new(Some(dir.path()));
        let texts = BTreeMap::from([
            ("a.ts".to_owned(), "a".to_owned()),
            ("b.ts".to_owned(), "a".to_owned()),
            ("c.ts".to_owned(), "c".to_owned()),
        ]);
        let manifest = snapshots.store(&texts);
        assert_eq!(manifest["a.ts"], manifest["b.ts"], "partagé par contenu");
        assert_eq!(snapshots.load(&manifest), texts);
        let keep: BTreeSet<String> = BTreeSet::from([manifest["a.ts"].clone()]);
        snapshots.collect(&keep);
        let reloaded = snapshots.load(&manifest);
        assert_eq!(reloaded["a.ts"], "a");
        assert_eq!(reloaded["c.ts"], "", "contenu supprimé");
    }
}
