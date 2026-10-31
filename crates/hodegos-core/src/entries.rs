//! Création et suppression de fichiers et de dossiers depuis l'arbre (phase 8a).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::ignore_rules::IgnoreRules;
use crate::{EntryKind, FileError, Project, RelPath, Session};

/// Fichier touché par une suppression ; un lien symbolique compte comme un fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedFile {
    pub path: RelPath,
    /// Exclu par `.gitignore` (ou sous un `.git` imbriqué), donc absent de l'arbre.
    pub ignored: bool,
}

/// Ce qu'une suppression emporte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletePlan {
    /// `File` pour un fichier ou un lien symbolique, même vers un dossier.
    pub kind: EntryKind,
    /// Fichiers supprimés, dossiers non compris, dans l'ordre alphabétique du parcours.
    pub files: Vec<DeletedFile>,
}

/// Entrée à créer ou à supprimer, sous un dossier parent résolu.
struct Target {
    /// Dossier parent réel, puis le nom tel qu'il est écrit : un lien n'est jamais suivi.
    path: PathBuf,
    /// Le même chemin relatif à la racine, pour lire les règles `.gitignore`.
    real_rel: RelPath,
}

impl Project {
    /// Crée un fichier vide ou un dossier, sans dossier intermédiaire, avec les droits
    /// par défaut de l'utilisateur. Refuse un nom déjà pris, la racine, `.git` et ce
    /// que `.gitignore` exclut.
    pub fn create_entry(
        &self,
        session: &Session,
        rel: &RelPath,
        kind: EntryKind,
    ) -> Result<(), FileError> {
        self.check(session)?;
        let target = self.target(rel)?;
        if self.is_excluded(rel, &target, kind == EntryKind::Dir) {
            return Err(FileError::Forbidden(rel.to_string()));
        }
        if fs::symlink_metadata(&target.path).is_ok() {
            return Err(FileError::AlreadyExists(rel.to_string()));
        }
        let created = match kind {
            EntryKind::File => fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target.path)
                .map(drop),
            EntryKind::Dir => fs::create_dir(&target.path),
        };
        created.map_err(|e| match e.kind() {
            io::ErrorKind::AlreadyExists => FileError::AlreadyExists(rel.to_string()),
            _ => FileError::io(rel.as_str(), e),
        })
    }

    /// Fichiers qu'emporterait la suppression de `rel`, fichiers ignorés compris.
    pub fn delete_preview(
        &self,
        session: &Session,
        rel: &RelPath,
    ) -> Result<DeletePlan, FileError> {
        self.check(session)?;
        let target = self.target(rel)?;
        self.plan(rel, &target)
    }

    /// Supprime `rel` pour de bon (pas de corbeille) et renvoie ce qui a été supprimé.
    /// Un lien symbolique est supprimé lui-même, jamais sa cible.
    pub fn delete_entry(&self, session: &Session, rel: &RelPath) -> Result<DeletePlan, FileError> {
        self.check(session)?;
        let target = self.target(rel)?;
        let plan = self.plan(rel, &target)?;
        match plan.kind {
            EntryKind::Dir => fs::remove_dir_all(&target.path),
            EntryKind::File => fs::remove_file(&target.path),
        }
        .map_err(|e| FileError::io(rel.as_str(), e))?;
        Ok(plan)
    }

    fn target(&self, rel: &RelPath) -> Result<Target, FileError> {
        let forbidden = || FileError::Forbidden(rel.to_string());
        let name = rel.components().last().ok_or_else(forbidden)?.to_owned();
        let parent = self.resolve_existing(&rel.parent())?;
        if !parent.is_dir {
            return Err(FileError::NotFound(rel.parent().to_string()));
        }
        let real_parent = parent
            .real
            .strip_prefix(self.root())
            .ok()
            .and_then(Path::to_str)
            .ok_or_else(forbidden)
            .and_then(RelPath::parse)?;
        Ok(Target {
            path: parent.real.join(&name),
            real_rel: real_parent.join(&name),
        })
    }

    /// Exclu par `.gitignore`, à son chemin écrit comme à son chemin réel.
    fn is_excluded(&self, rel: &RelPath, target: &Target, is_dir: bool) -> bool {
        IgnoreRules::for_dir(self.root(), &rel.parent()).is_ignored(&self.lexical(rel), is_dir)
            || IgnoreRules::for_dir(self.root(), &target.real_rel.parent())
                .is_ignored(&target.path, is_dir)
    }

    fn plan(&self, rel: &RelPath, target: &Target) -> Result<DeletePlan, FileError> {
        let meta =
            fs::symlink_metadata(&target.path).map_err(|e| FileError::io(rel.as_str(), e))?;
        let is_dir = meta.is_dir();
        if self.is_excluded(rel, target, is_dir) {
            return Err(FileError::Forbidden(rel.to_string()));
        }
        if !is_dir {
            return Ok(DeletePlan {
                kind: EntryKind::File,
                files: vec![DeletedFile {
                    path: rel.clone(),
                    ignored: false,
                }],
            });
        }
        let mut files = Vec::new();
        self.collect(&target.path, rel, &target.real_rel, false, &mut files)?;
        Ok(DeletePlan {
            kind: EntryKind::Dir,
            files,
        })
    }

    /// Parcourt un dossier sans suivre les liens ; sous un dossier ignoré, tout l'est.
    fn collect(
        &self,
        dir: &Path,
        rel: &RelPath,
        real_rel: &RelPath,
        ignored: bool,
        files: &mut Vec<DeletedFile>,
    ) -> Result<(), FileError> {
        let rules = (!ignored).then(|| IgnoreRules::for_dir(self.root(), real_rel));
        let read = fs::read_dir(dir).map_err(|e| FileError::io(rel.as_str(), e))?;
        let mut entries: Vec<fs::DirEntry> = read.filter_map(Result::ok).collect();
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
            let ignored = ignored
                || name == ".git"
                || rules.as_ref().is_some_and(|r| r.is_ignored(&path, is_dir));
            let child = rel.join(&name);
            if is_dir {
                self.collect(&path, &child, &real_rel.join(&name), ignored, files)?;
            } else {
                files.push(DeletedFile {
                    path: child,
                    ignored,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;
    use crate::UserId;

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn sample() -> (tempfile::TempDir, Project, Session) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, ".git/config", "[core]");
        write(root, ".gitignore", "target/\n*.log\n");
        write(root, "src/main.rs", "fn main() {}");
        write(root, "README.md", "# Troc");
        let project = Project::open(root).unwrap();
        let session = project.session(UserId::local());
        (dir, project, session)
    }

    fn rel(raw: &str) -> RelPath {
        RelPath::parse(raw).unwrap()
    }

    #[test]
    fn cree_un_fichier_vide_et_un_dossier() {
        let (dir, project, session) = sample();
        project
            .create_entry(&session, &rel("src/annuler.rs"), EntryKind::File)
            .unwrap();
        project
            .create_entry(&session, &rel("docs"), EntryKind::Dir)
            .unwrap();
        assert_eq!(fs::read(dir.path().join("src/annuler.rs")).unwrap(), b"");
        assert!(dir.path().join("docs").is_dir());
    }

    #[test]
    fn refuse_un_nom_pris_un_chemin_exclu_ou_un_parent_absent() {
        let (dir, project, session) = sample();
        write(dir.path(), "target/app", "bin");
        let cases = [
            ("src/main.rs", EntryKind::File, "pris"),
            ("src", EntryKind::Dir, "pris"),
            ("build.log", EntryKind::File, "exclu"),
            ("target", EntryKind::Dir, "exclu"),
            ("target/x.rs", EntryKind::File, "exclu"),
            ("absent/x.rs", EntryKind::File, "absent"),
            ("README.md/x", EntryKind::File, "absent"),
        ];
        for (raw, kind, expected) in cases {
            let err = project.create_entry(&session, &rel(raw), kind).unwrap_err();
            let ok = match expected {
                "pris" => matches!(err, FileError::AlreadyExists(_)),
                "exclu" => matches!(err, FileError::Forbidden(_)),
                _ => matches!(err, FileError::NotFound(_)),
            };
            assert!(ok, "{raw} : {err:?}");
        }
        assert!(!dir.path().join("build.log").exists());
        assert!(!dir.path().join("target/x.rs").exists());
        assert!(!dir.path().join("absent").exists());
        let err = project
            .create_entry(&session, &RelPath::root(), EntryKind::Dir)
            .unwrap_err();
        assert!(matches!(err, FileError::Forbidden(_)), "{err:?}");
    }

    #[test]
    fn refuse_de_creer_par_un_lien_qui_sort_du_projet() {
        let (dir, project, session) = sample();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), dir.path().join("dehors")).unwrap();
        let err = project
            .create_entry(&session, &rel("dehors/x.rs"), EntryKind::File)
            .unwrap_err();
        assert!(matches!(err, FileError::Forbidden(_)), "{err:?}");
        assert!(!outside.path().join("x.rs").exists());
    }

    #[test]
    fn un_lien_existant_compte_comme_un_nom_pris() {
        let (dir, project, session) = sample();
        symlink("/nulle/part", dir.path().join("casse")).unwrap();
        let err = project
            .create_entry(&session, &rel("casse"), EntryKind::File)
            .unwrap_err();
        assert!(matches!(err, FileError::AlreadyExists(_)), "{err:?}");
    }

    #[test]
    fn la_suppression_d_un_dossier_compte_les_fichiers_ignores_et_ne_suit_pas_les_liens() {
        let (dir, project, session) = sample();
        let root = dir.path();
        write(root, "app/src/lib.rs", "x");
        write(root, "app/.gitignore", "node_modules/\n");
        write(root, "app/node_modules/a/index.js", "x");
        write(root, "app/trace.log", "x");
        write(root, "app/vendor/.git/HEAD", "x");
        let outside = tempfile::tempdir().unwrap();
        write(outside.path(), "garde.txt", "à garder");
        symlink(outside.path(), root.join("app/dehors")).unwrap();

        let plan = project.delete_preview(&session, &rel("app")).unwrap();
        assert_eq!(plan.kind, EntryKind::Dir);
        let listed: Vec<(&str, bool)> = plan
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.ignored))
            .collect();
        assert_eq!(
            listed,
            [
                ("app/.gitignore", false),
                ("app/dehors", false),
                ("app/node_modules/a/index.js", true),
                ("app/src/lib.rs", false),
                ("app/trace.log", true),
                ("app/vendor/.git/HEAD", true),
            ]
        );
        assert!(root.join("app").exists(), "l'aperçu ne supprime rien");

        let deleted = project.delete_entry(&session, &rel("app")).unwrap();
        assert_eq!(deleted, plan);
        assert!(!root.join("app").exists());
        assert_eq!(
            fs::read_to_string(outside.path().join("garde.txt")).unwrap(),
            "à garder"
        );
    }

    #[test]
    fn supprimer_un_lien_vers_un_dossier_garde_sa_cible() {
        let (dir, project, session) = sample();
        symlink(dir.path().join("src"), dir.path().join("lien-src")).unwrap();
        let plan = project.delete_entry(&session, &rel("lien-src")).unwrap();
        assert_eq!(plan.kind, EntryKind::File);
        assert!(fs::symlink_metadata(dir.path().join("lien-src")).is_err());
        assert!(dir.path().join("src/main.rs").is_file());
    }

    #[test]
    fn refuse_de_supprimer_la_racine_un_chemin_exclu_ou_absent() {
        let (dir, project, session) = sample();
        write(dir.path(), "target/app", "bin");
        let err = project
            .delete_entry(&session, &RelPath::root())
            .unwrap_err();
        assert!(matches!(err, FileError::Forbidden(_)), "{err:?}");
        let err = project.delete_entry(&session, &rel("target")).unwrap_err();
        assert!(matches!(err, FileError::Forbidden(_)), "{err:?}");
        let err = project
            .delete_entry(&session, &rel("absent.rs"))
            .unwrap_err();
        assert!(matches!(err, FileError::NotFound(_)), "{err:?}");
        assert!(dir.path().join("target/app").exists());
    }
}
