//! Création et suppression de fichiers et de dossiers depuis l'arbre (phase 8a),
//! déplacement par glisser-déposer (sous-phase 8g).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rustix::fs::{CWD, RenameFlags, renameat_with};

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

/// Fichier qui change de chemin avec un déplacement ; un lien symbolique compte comme
/// un fichier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedFile {
    pub from: RelPath,
    pub to: RelPath,
}

/// Ce qu'un déplacement emporte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePlan {
    /// `File` pour un fichier ou un lien symbolique, même vers un dossier.
    pub kind: EntryKind,
    pub from: RelPath,
    pub to: RelPath,
    /// Fichiers présents sur le disque, dossiers non compris, ignorés compris.
    pub files: Vec<MovedFile>,
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

    /// Déplace `from` dans le dossier `to_dir` (renommage sur le disque, sans git), puis
    /// appelle `commit` avec ce qui a changé de chemin ; si `commit` échoue, le renommage
    /// est annulé et son erreur renvoyée. Un lien symbolique est déplacé lui-même. Refuse
    /// la racine, `.git`, ce que `.gitignore` exclut (avant comme après), une destination
    /// atteinte par un lien, un dossier dans lui-même, le même dossier et un nom pris :
    /// rien n'est jamais écrasé.
    pub fn move_entry(
        &self,
        session: &Session,
        from: &RelPath,
        to_dir: &RelPath,
        commit: impl FnOnce(&MovePlan) -> Result<(), FileError>,
    ) -> Result<MovePlan, FileError> {
        self.check(session)?;
        let source = self.target(from)?;
        let meta =
            fs::symlink_metadata(&source.path).map_err(|e| FileError::io(from.as_str(), e))?;
        let is_dir = meta.is_dir();
        if self.is_excluded(from, &source, is_dir) {
            return Err(FileError::Forbidden(from.to_string()));
        }
        if to_dir == from || to_dir.as_str().starts_with(&format!("{from}/")) {
            return Err(FileError::InvalidMove(format!(
                "{from} ne peut pas aller dans lui-même"
            )));
        }
        if *to_dir == from.parent() {
            return Err(FileError::InvalidMove(format!(
                "{from} est déjà dans ce dossier"
            )));
        }
        let dest = self.resolve_existing(to_dir)?;
        if !dest.is_dir {
            return Err(FileError::NotFound(to_dir.to_string()));
        }
        // Les remarques suivent le chemin écrit : il doit être le chemin réel.
        if dest.real != self.lexical(to_dir) {
            return Err(FileError::Forbidden(to_dir.to_string()));
        }
        let name = from.components().last().unwrap_or_default();
        let to = to_dir.join(name);
        let target = self.target(&to)?;
        if self.is_excluded(&to, &target, is_dir) {
            return Err(FileError::Forbidden(to.to_string()));
        }
        if fs::symlink_metadata(&target.path).is_ok() {
            return Err(FileError::AlreadyExists(to.to_string()));
        }
        let files = if is_dir {
            let mut found = Vec::new();
            self.collect(&source.path, from, &source.real_rel, false, &mut found)?;
            found
                .into_iter()
                .map(|file| MovedFile {
                    to: to.join(&file.path.as_str()[from.as_str().len() + 1..]),
                    from: file.path,
                })
                .collect()
        } else {
            vec![MovedFile {
                from: from.clone(),
                to: to.clone(),
            }]
        };
        let plan = MovePlan {
            kind: if is_dir {
                EntryKind::Dir
            } else {
                EntryKind::File
            },
            from: from.clone(),
            to: to.clone(),
            files,
        };
        rename_no_replace(&source.path, &target.path).map_err(|e| match e.kind() {
            io::ErrorKind::AlreadyExists => FileError::AlreadyExists(to.to_string()),
            _ => FileError::io(from.as_str(), e),
        })?;
        if let Err(err) = commit(&plan) {
            if let Err(undo) = rename_no_replace(&target.path, &source.path) {
                tracing::error!(%from, %to, error = %undo, "déplacement non annulé");
            }
            return Err(err);
        }
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

/// `rename` qui échoue si la destination existe, au lieu de l'écraser.
fn rename_no_replace(from: &Path, to: &Path) -> io::Result<()> {
    renameat_with(CWD, from, CWD, to, RenameFlags::NOREPLACE).map_err(io::Error::from)
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

    fn no_commit(_: &MovePlan) -> Result<(), FileError> {
        Ok(())
    }

    #[test]
    fn deplace_un_fichier_et_un_dossier_avec_ses_descendants() {
        let (dir, project, session) = sample();
        let root = dir.path();
        write(root, "docs/a.md", "a");
        write(root, "docs/adr/0001.md", "b");
        fs::create_dir(root.join("archive")).unwrap();

        let plan = project
            .move_entry(&session, &rel("README.md"), &rel("src"), no_commit)
            .unwrap();
        assert_eq!(plan.kind, EntryKind::File);
        assert_eq!(plan.to, rel("src/README.md"));
        assert_eq!(
            plan.files,
            [MovedFile {
                from: rel("README.md"),
                to: rel("src/README.md")
            }]
        );
        assert_eq!(
            fs::read_to_string(root.join("src/README.md")).unwrap(),
            "# Troc"
        );

        let mut seen = None;
        let plan = project
            .move_entry(&session, &rel("docs"), &rel("archive"), |plan| {
                seen = Some(plan.clone());
                Ok(())
            })
            .unwrap();
        assert_eq!(seen.as_ref(), Some(&plan));
        assert_eq!(plan.kind, EntryKind::Dir);
        let pairs: Vec<(&str, &str)> = plan
            .files
            .iter()
            .map(|f| (f.from.as_str(), f.to.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("docs/a.md", "archive/docs/a.md"),
                ("docs/adr/0001.md", "archive/docs/adr/0001.md"),
            ]
        );
        assert!(!root.join("docs").exists());
        assert!(root.join("archive/docs/adr/0001.md").is_file());

        project
            .move_entry(&session, &rel("archive/docs"), &RelPath::root(), no_commit)
            .unwrap();
        assert!(root.join("docs/a.md").is_file());
    }

    #[test]
    fn refuse_les_deplacements_interdits_sans_rien_toucher() {
        let (dir, project, session) = sample();
        let root = dir.path();
        write(root, "target/app", "bin");
        write(root, "app/src/lib.rs", "x");
        write(root, "docs/main.rs", "autre");
        write(root, "docs/notes.txt", "x");
        symlink(root.join("docs"), root.join("lien-docs")).unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), root.join("dehors")).unwrap();
        let cases = [
            ("", "src", "exclu"),
            ("target/app", "src", "exclu"),
            ("target", "src", "exclu"),
            ("README.md", "target", "exclu"),
            ("README.md", "dehors", "exclu"),
            ("README.md", "lien-docs", "exclu"),
            ("docs/notes.txt", "dehors", "exclu"),
            ("absent.rs", "src", "absent"),
            ("README.md", "absent", "absent"),
            ("README.md", "src/main.rs", "absent"),
            ("app", "app", "impossible"),
            ("app", "app/src", "impossible"),
            ("README.md", "", "impossible"),
            ("src/main.rs", "src", "impossible"),
            ("src/main.rs", "docs", "pris"),
        ];
        for (from, to_dir, expected) in cases {
            let from = if from.is_empty() {
                RelPath::root()
            } else {
                rel(from)
            };
            let err = project
                .move_entry(&session, &from, &rel(to_dir), no_commit)
                .unwrap_err();
            let ok = match expected {
                "exclu" => matches!(err, FileError::Forbidden(_)),
                "absent" => matches!(err, FileError::NotFound(_)),
                "impossible" => matches!(err, FileError::InvalidMove(_)),
                _ => matches!(err, FileError::AlreadyExists(_)),
            };
            assert!(ok, "{from} → {to_dir} : {err:?}");
        }
        write(root, "docs.log", "x");
        let err = project
            .move_entry(&session, &rel("docs.log"), &rel("src"), no_commit)
            .unwrap_err();
        assert!(matches!(err, FileError::Forbidden(_)), "{err:?}");
        write(root, "trace.txt", "x");
        write(root, "src/.gitignore", "trace.txt\n");
        let err = project
            .move_entry(&session, &rel("trace.txt"), &rel("src"), no_commit)
            .unwrap_err();
        assert!(matches!(err, FileError::Forbidden(_)), "{err:?}");
        assert!(root.join("trace.txt").is_file());
        assert_eq!(
            fs::read_to_string(root.join("docs/main.rs")).unwrap(),
            "autre"
        );
        assert_eq!(
            fs::read_to_string(root.join("src/main.rs")).unwrap(),
            "fn main() {}"
        );
        assert!(root.join("app/src/lib.rs").is_file());
        assert!(root.join("README.md").is_file());
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    }

    #[test]
    fn deplace_un_lien_lui_meme_et_garde_sa_cible() {
        let (dir, project, session) = sample();
        let root = dir.path();
        fs::create_dir(root.join("liens")).unwrap();
        symlink(root.join("src"), root.join("lien-src")).unwrap();
        let plan = project
            .move_entry(&session, &rel("lien-src"), &rel("liens"), no_commit)
            .unwrap();
        assert_eq!(plan.kind, EntryKind::File);
        assert_eq!(plan.files.len(), 1);
        assert!(
            fs::symlink_metadata(root.join("liens/lien-src"))
                .unwrap()
                .is_symlink()
        );
        assert!(fs::symlink_metadata(root.join("lien-src")).is_err());
        assert!(root.join("src/main.rs").is_file());
    }

    #[test]
    fn un_commit_qui_echoue_annule_le_renommage() {
        let (dir, project, session) = sample();
        let root = dir.path();
        write(root, "docs/a.md", "a");
        let err = project
            .move_entry(&session, &rel("docs"), &rel("src"), |_| {
                Err(FileError::Io {
                    path: "remarks.json".to_owned(),
                    source: io::Error::other("disque plein"),
                })
            })
            .unwrap_err();
        assert!(matches!(err, FileError::Io { .. }), "{err:?}");
        assert_eq!(fs::read_to_string(root.join("docs/a.md")).unwrap(), "a");
        assert!(!root.join("src/docs").exists());
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
