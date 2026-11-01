//! Ce que la carte sait de chaque fichier, avec son empreinte : après une modification,
//! seuls les fichiers touchés sont relus. Sérialisé dans `map.json`.

use std::collections::BTreeMap;
use std::fs;

use serde::{Deserialize, Serialize};

use super::extract::{FileFacts, facts};
use super::{Exclusions, MAX_MAP_FILES};
use crate::secrets::exclusion;
use crate::{FileError, Project, RelPath, Session};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MapCache {
    /// Chemin → empreinte et faits retenus.
    files: BTreeMap<String, Cached>,
    /// Exploration coupée à [`MAX_MAP_FILES`] fichiers.
    #[serde(default)]
    truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Cached {
    /// Empreinte du contenu (celle des versions de fichier).
    version: String,
    facts: FileFacts,
}

/// Bilan d'une mise à jour.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Refresh {
    /// Fichiers relus et analysés (contenu nouveau ou modifié).
    pub extracted: usize,
    /// Un fichier est apparu, a changé ou a disparu.
    pub changed: bool,
    /// Dossiers apparus, à observer.
    pub new_dirs: Vec<RelPath>,
}

impl MapCache {
    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub(crate) fn truncated(&self) -> bool {
        self.truncated
    }

    pub(crate) fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    pub(crate) fn facts(&self) -> impl Iterator<Item = (&str, &FileFacts)> {
        self.files.iter().map(|(path, c)| (path.as_str(), &c.facts))
    }

    /// Relit tout le projet ; un fichier dont l'empreinte n'a pas changé n'est pas
    /// réanalysé. À l'ouverture, et quand les règles d'exclusion changent.
    pub fn refresh(
        &mut self,
        project: &Project,
        session: &Session,
        exclude: &Exclusions,
    ) -> Result<Refresh, FileError> {
        project.check(session)?;
        let mut files = Vec::new();
        let mut dirs = Vec::new();
        let truncated = project.map_walk(
            &RelPath::root(),
            exclude,
            MAX_MAP_FILES,
            &mut files,
            &mut dirs,
        )?;
        let mut refresh = Refresh {
            changed: truncated != self.truncated,
            ..Refresh::default()
        };
        self.truncated = truncated;
        let keep: std::collections::BTreeSet<&str> = files.iter().map(RelPath::as_str).collect();
        let before = self.files.len();
        self.files.retain(|path, _| keep.contains(path.as_str()));
        refresh.changed |= self.files.len() != before;
        for rel in &files {
            self.update(project, session, rel, &mut refresh);
        }
        Ok(refresh)
    }

    /// Met à jour les chemins touchés : un fichier est relu, un dossier reparcouru, un
    /// chemin disparu (ou désormais ignoré) retiré avec tout ce qu'il contenait. Un
    /// `.gitignore` modifié fait tout relire.
    pub fn refresh_paths(
        &mut self,
        project: &Project,
        session: &Session,
        exclude: &Exclusions,
        paths: &[RelPath],
    ) -> Result<Refresh, FileError> {
        project.check(session)?;
        if paths.iter().any(|p| p.as_str().ends_with(".gitignore")) {
            return self.refresh(project, session, exclude);
        }
        let mut refresh = Refresh::default();
        for rel in paths {
            if rel.is_root() || rel.components().any(|part| part == ".git") {
                continue;
            }
            let meta = fs::symlink_metadata(project.lexical(rel)).ok();
            let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
            let is_file = meta.as_ref().is_some_and(|m| m.is_file());
            if !(is_dir || is_file) || !project.map_admits(rel, is_dir, exclude) {
                refresh.changed |= self.remove_under(rel);
                continue;
            }
            if is_file {
                self.update(project, session, rel, &mut refresh);
                continue;
            }
            let mut files = Vec::new();
            let mut dirs = vec![rel.clone()];
            let room = MAX_MAP_FILES.saturating_sub(self.files.len());
            let cut = project.map_walk(rel, exclude, room, &mut files, &mut dirs)?;
            self.truncated |= cut;
            let keep: std::collections::BTreeSet<&str> =
                files.iter().map(RelPath::as_str).collect();
            let prefix = format!("{}/", rel.as_str());
            let before = self.files.len();
            self.files
                .retain(|path, _| !path.starts_with(&prefix) || keep.contains(path.as_str()));
            refresh.changed |= self.files.len() != before;
            for file in &files {
                self.update(project, session, file, &mut refresh);
            }
            refresh.new_dirs.extend(dirs);
        }
        Ok(refresh)
    }

    /// Relit `rel` ; illisible, binaire ou porteur d'une clé privée : retiré.
    fn update(&mut self, project: &Project, session: &Session, rel: &RelPath, out: &mut Refresh) {
        let file = match project.read_file(session, rel) {
            Ok(file) if exclusion(rel.as_str(), &file.content).is_none() => file,
            _ => {
                out.changed |= self.files.remove(rel.as_str()).is_some();
                return;
            }
        };
        let version = file.version.as_str();
        if self
            .files
            .get(rel.as_str())
            .is_some_and(|c| c.version == version)
        {
            return;
        }
        if !self.files.contains_key(rel.as_str()) && self.files.len() >= MAX_MAP_FILES {
            self.truncated = true;
            return;
        }
        self.files.insert(
            rel.as_str().to_owned(),
            Cached {
                version: version.to_owned(),
                facts: facts(rel.as_str(), &file.content),
            },
        );
        out.extracted += 1;
        out.changed = true;
    }

    /// Retire `rel` et ce qu'il contient ; vrai si quelque chose a disparu.
    fn remove_under(&mut self, rel: &RelPath) -> bool {
        let prefix = format!("{}/", rel.as_str());
        let before = self.files.len();
        self.files
            .retain(|path, _| path != rel.as_str() && !path.starts_with(&prefix));
        self.files.len() != before
    }
}
