use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::Registry;

#[derive(Debug, thiserror::Error)]
pub enum EjectError {
    #[error("aucun mentor actif ne porte l'id « {0} » (voir hodegos mentors list)")]
    Unknown(String),
    #[error("{0} existe déjà ; modifiez-le directement ou supprimez-le d'abord")]
    Exists(PathBuf),
    #[error("écriture impossible dans {path} : {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Copie le fichier du mentor actif `id` dans `dest_dir/<id>.md`, prêt à être modifié.
pub fn eject(registry: &Registry, id: &str, dest_dir: &Path) -> Result<PathBuf, EjectError> {
    let mentor = registry
        .get(id)
        .ok_or_else(|| EjectError::Unknown(id.to_owned()))?;
    let dest = dest_dir.join(format!("{id}.md"));
    let io = |source| EjectError::Io {
        path: dest.clone(),
        source,
    };
    fs::create_dir_all(dest_dir).map_err(io)?;
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&dest)
    {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(EjectError::Exists(dest));
        }
        Err(err) => return Err(io(err)),
    };
    file.write_all(mentor.source_text.as_bytes()).map_err(io)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MentorSources, Origin};

    #[test]
    fn copie_un_mentor_livre_puis_refuse_d_ecraser() {
        let dir = tempfile::tempdir().unwrap();
        let sources = MentorSources {
            user_dir: Some(dir.path().join("user")),
            project_dir: None,
        };
        let registry = Registry::load(&sources);
        let dest = eject(&registry, "architecte", sources.user_dir.as_ref().unwrap()).unwrap();
        assert_eq!(
            fs::read_to_string(&dest).unwrap(),
            include_str!("../../../mentors/architecte.md")
        );
        let err = eject(&registry, "architecte", sources.user_dir.as_ref().unwrap()).unwrap_err();
        assert!(matches!(err, EjectError::Exists(_)), "{err:?}");

        let reloaded = Registry::load(&sources);
        assert_eq!(reloaded.get("architecte").unwrap().origin, Origin::User);
    }

    #[test]
    fn refuse_un_mentor_inconnu() {
        let dir = tempfile::tempdir().unwrap();
        let registry = Registry::load(&MentorSources::default());
        let err = eject(&registry, "inconnu", dir.path()).unwrap_err();
        assert!(matches!(err, EjectError::Unknown(_)), "{err:?}");
    }
}
