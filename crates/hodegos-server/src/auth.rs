use hodegos_core::UserId;

use crate::ServerError;
use crate::secret::Secret;

/// Authentification interchangeable : le MVP vérifie le jeton de lancement,
/// une version multi-utilisateur pourra brancher autre chose.
pub trait Authenticator: Send + Sync + 'static {
    /// Échange un jeton présenté dans l'URL contre la valeur du cookie de session.
    fn exchange(&self, token: &str) -> Option<String>;

    /// Identifie la personne à partir de la valeur du cookie de session.
    fn authenticate(&self, session: &str) -> Option<UserId>;
}

/// Jeton de lancement tiré à chaque démarrage, réutilisable tant que le processus
/// tourne, échangé contre un secret de session distinct.
#[derive(Debug)]
pub struct LaunchTokenAuthenticator {
    token: Secret,
    session: Secret,
}

impl LaunchTokenAuthenticator {
    pub fn generate() -> Result<Self, ServerError> {
        Ok(Self {
            token: Secret::generate()?,
            session: Secret::generate()?,
        })
    }

    pub fn launch_token(&self) -> &str {
        self.token.as_str()
    }
}

impl Authenticator for LaunchTokenAuthenticator {
    fn exchange(&self, token: &str) -> Option<String> {
        self.token
            .matches(token)
            .then(|| self.session.as_str().to_owned())
    }

    fn authenticate(&self, session: &str) -> Option<UserId> {
        self.session.matches(session).then(UserId::local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_jeton_s_echange_contre_une_session_distincte_et_reutilisable() {
        let auth = LaunchTokenAuthenticator::generate().unwrap();
        let token = auth.launch_token().to_owned();
        let session = auth.exchange(&token).unwrap();
        assert_ne!(session, token);
        assert_eq!(auth.exchange(&token), Some(session.clone()));
        assert_eq!(auth.authenticate(&session), Some(UserId::local()));
    }

    #[test]
    fn refuse_un_mauvais_jeton_et_le_jeton_comme_session() {
        let auth = LaunchTokenAuthenticator::generate().unwrap();
        assert_eq!(auth.exchange("faux"), None);
        assert_eq!(auth.authenticate(auth.launch_token()), None);
    }
}
