use std::io;

use crate::files::Version;

/// Erreur d'une opération sur les fichiers du projet.
#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("chemin refusé : {0}")]
    Forbidden(String),
    #[error("introuvable : {0}")]
    NotFound(String),
    #[error("{0} n'est pas un texte UTF-8 de 2 Mo au plus")]
    NotText(String),
    #[error("{path} a changé sur le disque depuis son ouverture")]
    Conflict { path: String, current: Version },
    #[error("cette session ne concerne pas ce projet")]
    WrongSession,
    #[error("observation des fichiers impossible : {0}")]
    Watch(String),
    #[error("erreur d'accès à {path} : {source}")]
    Io {
        path: String,
        #[source]
        source: io::Error,
    },
}

impl FileError {
    pub(crate) fn io(path: &str, source: io::Error) -> Self {
        match source.kind() {
            io::ErrorKind::NotFound => Self::NotFound(path.to_owned()),
            _ => Self::Io {
                path: path.to_owned(),
                source,
            },
        }
    }
}
