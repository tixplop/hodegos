use std::io;
use std::path::{Component, Path, PathBuf};

use crate::ignore_rules::IgnoreRules;
use crate::{FileError, ProjectId, RelPath, Session, UserId};

/// Racine sous laquelle WSL2 monte les disques Windows.
const WINDOWS_MOUNT_ROOT: &str = "/mnt";

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("le dossier du projet {path} est introuvable : {source}")]
    NotFound {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{0} n'est pas un dossier")]
    NotADirectory(PathBuf),
    #[error(
        "le projet {0} est sur un disque Windows (/mnt/), où les modifications de fichiers \
         ne sont pas détectées ; déplacez-le dans le système de fichiers Linux (~/…) \
         puis relancez Hodegos"
    )]
    WindowsMount(PathBuf),
    #[error("le dossier {0} existe déjà et n'est pas vide")]
    NotEmpty(PathBuf),
    #[error("impossible de créer le dossier {path} : {source}")]
    Create {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Un projet ouvert dans Hodegos : un dossier du système de fichiers Linux.
#[derive(Debug, Clone)]
pub struct Project {
    root: PathBuf,
    id: ProjectId,
}

/// Chemin existant du projet, validé.
#[derive(Debug)]
pub(crate) struct Resolved {
    /// Chemin réel, liens symboliques résolus, lui aussi sous la racine.
    pub(crate) real: PathBuf,
    pub(crate) is_dir: bool,
}

impl Project {
    /// Crée le dossier d'un nouveau projet (`hodegos new`) et renvoie son chemin réel.
    ///
    /// Refuse un chemin sous `/mnt/`, tel qu'écrit comme après résolution de son plus
    /// proche parent existant, et un dossier existant non vide ; rien n'est créé
    /// en cas de refus. Un dossier existant vide est accepté.
    pub fn create(path: &Path) -> Result<PathBuf, ProjectError> {
        let create_error = |source| ProjectError::Create {
            path: path.to_owned(),
            source,
        };
        let absolute = std::path::absolute(path).map_err(create_error)?;
        if is_windows_mount(&absolute) {
            return Err(ProjectError::WindowsMount(absolute));
        }
        let mut existing = absolute.as_path();
        let mut missing = Vec::new();
        while !existing.exists() {
            let (Some(parent), Some(name)) = (existing.parent(), existing.file_name()) else {
                break;
            };
            missing.push(name.to_owned());
            existing = parent;
        }
        let mut resolved = existing.canonicalize().map_err(create_error)?;
        resolved.extend(missing.iter().rev());
        if is_windows_mount(&resolved) {
            return Err(ProjectError::WindowsMount(resolved));
        }
        if resolved.exists() {
            if !resolved.is_dir() {
                return Err(ProjectError::NotADirectory(resolved));
            }
            let mut entries = std::fs::read_dir(&resolved).map_err(create_error)?;
            if entries.next().is_some() {
                return Err(ProjectError::NotEmpty(resolved));
            }
        }
        std::fs::create_dir_all(&resolved).map_err(create_error)?;
        resolved.canonicalize().map_err(create_error)
    }

    /// Valide le dossier du projet et résout son chemin (liens symboliques compris).
    ///
    /// Refuse un chemin sous `/mnt/`, avant comme après résolution.
    pub fn open(path: &Path) -> Result<Self, ProjectError> {
        let not_found = |source| ProjectError::NotFound {
            path: path.to_owned(),
            source,
        };
        let absolute = std::path::absolute(path).map_err(not_found)?;
        if is_windows_mount(&absolute) {
            return Err(ProjectError::WindowsMount(absolute));
        }
        let root = absolute.canonicalize().map_err(not_found)?;
        if is_windows_mount(&root) {
            return Err(ProjectError::WindowsMount(root));
        }
        if !root.is_dir() {
            return Err(ProjectError::NotADirectory(root));
        }
        let id = ProjectId::from_root(&root);
        Ok(Self { root, id })
    }

    pub fn id(&self) -> &ProjectId {
        &self.id
    }

    /// Session de `user` sur ce projet.
    pub fn session(&self, user: UserId) -> Session {
        Session {
            user,
            project: self.id.clone(),
        }
    }

    /// Chemin absolu et résolu de la racine du projet.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Project {
    pub(crate) fn check(&self, session: &Session) -> Result<(), FileError> {
        if session.project == self.id {
            Ok(())
        } else {
            Err(FileError::WrongSession)
        }
    }

    pub(crate) fn lexical(&self, rel: &RelPath) -> PathBuf {
        if rel.is_root() {
            self.root.clone()
        } else {
            self.root.join(rel.as_str())
        }
    }

    /// Résout un chemin existant. Refuse ce qui sort de la racine (liens symboliques
    /// compris), ce qui passe par `.git` et ce que `.gitignore` exclut.
    pub(crate) fn resolve_existing(&self, rel: &RelPath) -> Result<Resolved, FileError> {
        self.resolve_with(rel, &IgnoreRules::for_dir(&self.root, &rel.parent()))
    }

    /// Comme [`Self::resolve_existing`], avec les règles déjà lues du dossier parent.
    pub(crate) fn resolve_with(
        &self,
        rel: &RelPath,
        parent_rules: &IgnoreRules,
    ) -> Result<Resolved, FileError> {
        let forbidden = || FileError::Forbidden(rel.to_string());
        let lexical = self.lexical(rel);
        let real = lexical
            .canonicalize()
            .map_err(|e| FileError::io(rel.as_str(), e))?;
        let inside = real.strip_prefix(&self.root).map_err(|_| forbidden())?;
        if inside
            .components()
            .any(|c| matches!(c, Component::Normal(name) if name == ".git"))
        {
            return Err(forbidden());
        }
        let is_dir = real.is_dir();
        if !rel.is_root() {
            if parent_rules.is_ignored(&lexical, is_dir) {
                return Err(forbidden());
            }
            if real != lexical {
                let real_rel = inside
                    .to_str()
                    .ok_or_else(forbidden)
                    .and_then(RelPath::parse)?;
                if !real_rel.is_root()
                    && IgnoreRules::for_dir(&self.root, &real_rel.parent())
                        .is_ignored(&real, is_dir)
                {
                    return Err(forbidden());
                }
            }
        }
        Ok(Resolved { real, is_dir })
    }
}

fn is_windows_mount(path: &Path) -> bool {
    path.starts_with(WINDOWS_MOUNT_ROOT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ouvre_un_dossier_et_resout_son_chemin() {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::open(dir.path()).unwrap();
        assert_eq!(project.root(), dir.path().canonicalize().unwrap());
    }

    #[test]
    fn create_accepte_un_dossier_absent_ou_vide_et_refuse_un_dossier_non_vide() {
        let dir = tempfile::tempdir().unwrap();
        let created = Project::create(&dir.path().join("a/b")).unwrap();
        assert_eq!(created, dir.path().canonicalize().unwrap().join("a/b"));
        assert!(created.is_dir());
        assert_eq!(
            Project::create(&created).unwrap(),
            created,
            "vide : accepté"
        );
        std::fs::write(created.join("x"), "").unwrap();
        assert!(matches!(
            Project::create(&created),
            Err(ProjectError::NotEmpty(_))
        ));
        assert!(matches!(
            Project::create(&created.join("x")),
            Err(ProjectError::NotADirectory(_))
        ));
    }

    #[test]
    fn create_refuse_mnt_meme_par_un_lien_sans_rien_creer() {
        assert!(matches!(
            Project::create(Path::new("/mnt/c/hodegos-inexistant/projet")),
            Err(ProjectError::WindowsMount(_))
        ));
        assert!(!Path::new("/mnt/c/hodegos-inexistant").exists());
        if Path::new(WINDOWS_MOUNT_ROOT).is_dir() {
            let dir = tempfile::tempdir().unwrap();
            std::os::unix::fs::symlink(WINDOWS_MOUNT_ROOT, dir.path().join("lien")).unwrap();
            let target = dir.path().join("lien/hodegos-inexistant/projet");
            assert!(matches!(
                Project::create(&target),
                Err(ProjectError::WindowsMount(_))
            ));
            assert!(!Path::new("/mnt/hodegos-inexistant").exists());
        }
    }

    #[test]
    fn refuse_un_chemin_sous_mnt_meme_inexistant() {
        let err = Project::open(Path::new("/mnt/c/hodegos-inexistant")).unwrap_err();
        assert!(matches!(err, ProjectError::WindowsMount(_)), "{err:?}");
        assert!(err.to_string().contains("/mnt/"));
    }

    #[test]
    fn refuse_mnt_lui_meme_mais_pas_un_nom_proche() {
        assert!(is_windows_mount(Path::new("/mnt")));
        assert!(is_windows_mount(Path::new("/mnt/c/Users")));
        assert!(!is_windows_mount(Path::new("/mntx/projet")));
        assert!(!is_windows_mount(Path::new("/home/moi/mnt/projet")));
    }

    #[test]
    fn refuse_un_lien_symbolique_vers_mnt() {
        if !Path::new(WINDOWS_MOUNT_ROOT).is_dir() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("projet");
        std::os::unix::fs::symlink(WINDOWS_MOUNT_ROOT, &link).unwrap();
        let err = Project::open(&link).unwrap_err();
        assert!(matches!(err, ProjectError::WindowsMount(_)), "{err:?}");
    }

    #[test]
    fn refuse_un_dossier_inexistant() {
        let dir = tempfile::tempdir().unwrap();
        let err = Project::open(&dir.path().join("absent")).unwrap_err();
        assert!(matches!(err, ProjectError::NotFound { .. }), "{err:?}");
    }

    #[test]
    fn refuse_un_fichier() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let err = Project::open(file.path()).unwrap_err();
        assert!(matches!(err, ProjectError::NotADirectory(_)), "{err:?}");
    }
}
