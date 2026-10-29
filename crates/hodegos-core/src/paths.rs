use std::fmt;

use crate::FileError;

/// Chemin relatif à la racine du projet, normalisé et séparé par `/` ; vide pour la racine.
///
/// La construction refuse les chemins absolus, `..` et tout passage par `.git`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelPath(String);

impl RelPath {
    pub fn root() -> Self {
        Self(String::new())
    }

    pub fn parse(raw: &str) -> Result<Self, FileError> {
        let forbidden = || FileError::Forbidden(raw.to_owned());
        if raw.starts_with('/') || raw.contains('\0') {
            return Err(forbidden());
        }
        let mut parts = Vec::new();
        for part in raw.split('/') {
            match part {
                "" | "." => {}
                ".." | ".git" => return Err(forbidden()),
                part => parts.push(part),
            }
        }
        Ok(Self(parts.join("/")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn components(&self) -> impl Iterator<Item = &str> {
        self.0.split('/').filter(|part| !part.is_empty())
    }

    /// Dossier parent ; la racine pour un élément de premier niveau.
    pub fn parent(&self) -> Self {
        match self.0.rsplit_once('/') {
            Some((parent, _)) => Self(parent.to_owned()),
            None => Self::root(),
        }
    }

    pub fn join(&self, name: &str) -> Self {
        if self.is_root() {
            Self(name.to_owned())
        } else {
            Self(format!("{}/{name}", self.0))
        }
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_les_chemins_relatifs() {
        for (raw, expected) in [
            ("", ""),
            (".", ""),
            ("src/main.rs", "src/main.rs"),
            ("./src//main.rs/", "src/main.rs"),
            ("docs/./adr/0001.md", "docs/adr/0001.md"),
        ] {
            assert_eq!(RelPath::parse(raw).unwrap().as_str(), expected, "{raw}");
        }
    }

    #[test]
    fn refuse_les_chemins_qui_sortent_ou_visent_git() {
        for raw in [
            "/etc/passwd",
            "../voisin",
            "src/../../x",
            ".git/config",
            "sous/.git/HEAD",
            "a\0b",
        ] {
            assert!(
                matches!(RelPath::parse(raw), Err(FileError::Forbidden(_))),
                "{raw:?}"
            );
        }
    }

    #[test]
    fn parent_et_jointure() {
        let path = RelPath::parse("docs/adr/0001.md").unwrap();
        assert_eq!(path.parent().as_str(), "docs/adr");
        assert_eq!(RelPath::parse("a.md").unwrap().parent(), RelPath::root());
        assert_eq!(
            RelPath::root().join("src").join("lib.rs").as_str(),
            "src/lib.rs"
        );
    }
}
