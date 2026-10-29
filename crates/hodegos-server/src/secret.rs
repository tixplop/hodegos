use std::fmt;

use subtle::ConstantTimeEq;

use crate::ServerError;

/// Secret aléatoire de 256 bits, en hexadécimal ; jamais affiché par `Debug`.
pub(crate) struct Secret(String);

impl Secret {
    pub(crate) fn generate() -> Result<Self, ServerError> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|e| ServerError::Random(e.to_string()))?;
        Ok(Self(bytes.iter().map(|b| format!("{b:02x}")).collect()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Comparaison à temps constant.
    pub(crate) fn matches(&self, candidate: &str) -> bool {
        self.0.as_bytes().ct_eq(candidate.as_bytes()).into()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(…)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_secret_fait_256_bits_en_hexadecimal() {
        let secret = Secret::generate().unwrap();
        assert_eq!(secret.as_str().len(), 64);
        assert!(secret.as_str().bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn deux_secrets_different() {
        assert_ne!(
            Secret::generate().unwrap().as_str(),
            Secret::generate().unwrap().as_str()
        );
    }

    #[test]
    fn ne_reconnait_que_lui_meme() {
        let secret = Secret::generate().unwrap();
        assert!(secret.matches(secret.as_str()));
        assert!(!secret.matches(""));
        assert!(!secret.matches(&secret.as_str()[..63]));
        assert!(!format!("{secret:?}").contains(secret.as_str()));
    }
}
