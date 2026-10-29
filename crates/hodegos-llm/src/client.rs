use std::future::Future;
use std::pin::Pin;

use serde::Serialize;

/// Futur renvoyé par [`LlmClient::complete`], pour que le trait reste utilisable en `dyn`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}

/// Demande au modèle, avec le schéma JSON que doit suivre la réponse.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub schema_name: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmResponse {
    /// Contenu brut renvoyé par le modèle, censé être le JSON demandé.
    pub content: String,
    /// Coût de l'appel en dollars (crédits OpenRouter).
    pub cost_usd: f64,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LlmError {
    #[error("délai dépassé chez le fournisseur de modèles")]
    Timeout,
    #[error("le fournisseur a répondu {status} : {message}")]
    Http { status: u16, message: String },
    #[error("fournisseur injoignable : {0}")]
    Transport(String),
    #[error("réponse du fournisseur illisible : {0}")]
    InvalidResponse(String),
}

/// Accès à un modèle de langage. Les tests utilisent [`crate::FakeLlmClient`].
pub trait LlmClient: Send + Sync {
    fn complete<'a>(
        &'a self,
        request: &'a LlmRequest,
    ) -> BoxFuture<'a, Result<LlmResponse, LlmError>>;
}
