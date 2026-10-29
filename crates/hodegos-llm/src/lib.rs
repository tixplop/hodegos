//! Accès aux modèles : trait `LlmClient`, client OpenRouter, faux client pour les
//! tests, validation des remarques, configuration et budget mensuel.
//!
//! Indépendant des autres crates de Hodegos.

mod ask;
mod budget;
mod client;
mod config;
mod fake;
mod openrouter;
mod validate;

pub use ask::{AskError, Asked, ask_for_remarks};
pub use budget::{Budget, BudgetLevel, BudgetState, month_of};
pub use client::{BoxFuture, ChatMessage, LlmClient, LlmError, LlmRequest, LlmResponse, Role};
pub use config::{ApiKey, BudgetConfig, ConfigError, LlmConfig, load_api_key};
pub use fake::FakeLlmClient;
pub use openrouter::OpenRouterClient;
pub use validate::{
    MAX_MESSAGE_CHARS, MIN_CONFIDENCE, RemarkDraft, RemarkKind, RemarkLevel, Rules, ValidRemark,
    parse_remarks, remarks_schema, validate,
};
