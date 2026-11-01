//! Fichiers que la carte peut lire : ceux de l'arbre (ni `.git`, ni ignorés, ni liens
//! symboliques), moins les fichiers de secrets et les exclusions du projet.

use std::fs;

use super::Exclusions;
use crate::ignore_rules::IgnoreRules;
use crate::secrets::exclusion;
use crate::{FileError, Project, RelPath};

impl Project {
    /// Parcourt `from` (racine comprise) dans l'ordre de l'arbre : fichiers retenus dans
    /// `files` (au plus `limit` au total), dossiers dans `dirs`. Vrai si la liste est coupée.
    pub(crate) fn map_walk(
        &self,
        from: &RelPath,
        exclude: &Exclusions,
        limit: usize,
        files: &mut Vec<RelPath>,
        dirs: &mut Vec<RelPath>,
    ) -> Result<bool, FileError> {
        let rules = IgnoreRules::for_dir(self.root(), from);
        let read = fs::read_dir(self.lexical(from)).map_err(|e| FileError::io(from.as_str(), e))?;
        let mut entries: Vec<(String, bool)> = read
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let kind = entry.file_type().ok()?;
                if kind.is_symlink() || !(kind.is_dir() || kind.is_file()) {
                    return None;
                }
                let name = entry.file_name().into_string().ok()?;
                let ignored = rules.is_ignored(&entry.path(), kind.is_dir());
                (name != ".git" && !ignored).then_some((name, kind.is_dir()))
            })
            .collect();
        entries.sort_by(|a, b| {
            a.0.to_lowercase()
                .cmp(&b.0.to_lowercase())
                .then_with(|| a.0.cmp(&b.0))
        });
        for (name, is_dir) in entries {
            let child = from.join(&name);
            if exclude.excludes(child.as_str()) {
                continue;
            }
            if is_dir {
                dirs.push(child.clone());
                if self.map_walk(&child, exclude, limit, files, dirs)? {
                    return Ok(true);
                }
            } else if exclusion(child.as_str(), "").is_none() {
                if files.len() == limit {
                    return Ok(true);
                }
                files.push(child);
            }
        }
        Ok(false)
    }

    /// `rel` peut entrer dans la carte : ni sous `.git`, ni ignoré, ni exclu, ni lien
    /// symbolique, ni (pour un fichier) fichier de secrets par son nom.
    pub(crate) fn map_admits(&self, rel: &RelPath, is_dir: bool, exclude: &Exclusions) -> bool {
        if rel.is_root() || rel.components().any(|part| part == ".git") {
            return false;
        }
        if exclude.excludes(rel.as_str()) {
            return false;
        }
        if !is_dir && exclusion(rel.as_str(), "").is_some() {
            return false;
        }
        let path = self.lexical(rel);
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return false;
        }
        !IgnoreRules::for_dir(self.root(), &rel.parent()).is_ignored(&path, is_dir)
    }
}
