use std::fmt;

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
