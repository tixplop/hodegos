use std::fmt;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use sha2::{Digest, Sha256};

/// Identifiant de la personne qui utilise Hodegos.
///
/// En MVP, il n'y a qu'un utilisateur : [`UserId::local`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(String);

impl UserId {
    /// L'unique utilisateur du MVP, sur sa propre machine.
    pub fn local() -> Self {
        Self("local".to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identifiant opaque d'un projet, dérivé du chemin résolu de sa racine.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectId(String);

impl ProjectId {
    pub(crate) fn from_root(root: &Path) -> Self {
        let digest = Sha256::digest(root.as_os_str().as_bytes());
        Self(digest[..8].iter().map(|b| format!("{b:02x}")).collect())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Contexte explicite de chaque appel du cœur : qui agit, sur quel projet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub user: UserId,
    pub project: ProjectId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_identifiant_de_projet_est_stable_et_depend_du_chemin() {
        let a = ProjectId::from_root(Path::new("/home/moi/troc"));
        assert_eq!(a, ProjectId::from_root(Path::new("/home/moi/troc")));
        assert_ne!(a, ProjectId::from_root(Path::new("/home/moi/autre")));
        assert_eq!(a.as_str().len(), 16);
    }
}
