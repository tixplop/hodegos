use std::fs;

use crate::ignore_rules::IgnoreRules;
use crate::{FileError, Project, RelPath, Session};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Dir,
    File,
}

/// Entrée d'un dossier du projet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
}

impl Project {
    /// Contenu d'un dossier, sans `.git`, sans ce que `.gitignore` exclut, sans les liens
    /// qui sortent du projet ; dossiers d'abord, puis ordre alphabétique sans casse.
    pub fn list_dir(&self, session: &Session, rel: &RelPath) -> Result<Vec<DirEntry>, FileError> {
        self.check(session)?;
        let dir = self.resolve_existing(rel)?;
        if !dir.is_dir {
            return Err(FileError::NotFound(rel.to_string()));
        }
        let rules = IgnoreRules::for_dir(self.root(), rel);
        let read = fs::read_dir(&dir.real).map_err(|e| FileError::io(rel.as_str(), e))?;

        let mut entries = Vec::new();
        for entry in read.filter_map(Result::ok) {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if name == ".git" {
                continue;
            }
            let child = rel.join(&name);
            let Ok(resolved) = self.resolve_with(&child, &rules) else {
                continue;
            };
            let kind = if resolved.is_dir {
                EntryKind::Dir
            } else {
                EntryKind::File
            };
            entries.push(DirEntry { name, kind });
        }
        entries.sort_by(|a, b| {
            (a.kind != EntryKind::Dir)
                .cmp(&(b.kind != EntryKind::Dir))
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::UserId;

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn names(project: &Project, rel: &str) -> Vec<String> {
        let session = project.session(UserId::local());
        project
            .list_dir(&session, &RelPath::parse(rel).unwrap())
            .unwrap()
            .into_iter()
            .map(|e| match e.kind {
                EntryKind::Dir => format!("{}/", e.name),
                EntryKind::File => e.name,
            })
            .collect()
    }

    fn sample() -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, ".git/config", "[core]");
        write(root, ".git/info/exclude", "local.txt\n");
        write(root, ".gitignore", "target/\n.env\n*.log\n");
        write(root, ".env", "SECRET=1");
        write(root, "local.txt", "x");
        write(root, "target/debug/app", "bin");
        write(root, "README.md", "# Troc");
        write(root, "build.log", "x");
        write(root, "src/main.rs", "fn main() {}");
        write(root, "src/.gitignore", "genere.rs\n!garde.log\n");
        write(root, "src/genere.rs", "x");
        write(root, "src/garde.log", "x");
        write(root, "docs/adr/0001.md", "# ADR");
        write(root, "Zeta.txt", "x");
        write(root, "alpha.txt", "x");
        let project = Project::open(root).unwrap();
        (dir, project)
    }

    #[test]
    fn la_racine_exclut_git_et_les_fichiers_ignores() {
        let (_dir, project) = sample();
        assert_eq!(
            names(&project, ""),
            [
                "docs/",
                "src/",
                ".gitignore",
                "alpha.txt",
                "README.md",
                "Zeta.txt"
            ]
        );
    }

    #[test]
    fn un_gitignore_de_sous_dossier_s_applique_et_peut_reinclure() {
        let (_dir, project) = sample();
        assert_eq!(
            names(&project, "src"),
            [".gitignore", "garde.log", "main.rs"]
        );
    }

    #[test]
    fn un_dossier_ignore_ou_git_est_refuse() {
        let (_dir, project) = sample();
        let session = project.session(UserId::local());
        for rel in ["target", "target/debug"] {
            let err = project
                .list_dir(&session, &RelPath::parse(rel).unwrap())
                .unwrap_err();
            assert!(matches!(err, FileError::Forbidden(_)), "{rel} : {err:?}");
        }
    }

    #[test]
    fn un_lien_qui_sort_du_projet_est_masque_et_refuse() {
        let (dir, project) = sample();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("dehors")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("src"), dir.path().join("lien-src")).unwrap();
        let listed = names(&project, "");
        assert!(!listed.contains(&"dehors/".to_owned()), "{listed:?}");
        assert!(listed.contains(&"lien-src/".to_owned()), "{listed:?}");
        let session = project.session(UserId::local());
        let err = project
            .list_dir(&session, &RelPath::parse("dehors").unwrap())
            .unwrap_err();
        assert!(matches!(err, FileError::Forbidden(_)), "{err:?}");
    }

    #[test]
    fn refuse_une_session_d_un_autre_projet() {
        let (_dir, project) = sample();
        let (_other_dir, other) = sample();
        let session = other.session(UserId::local());
        let err = project.list_dir(&session, &RelPath::root()).unwrap_err();
        assert!(matches!(err, FileError::WrongSession), "{err:?}");
    }
}
