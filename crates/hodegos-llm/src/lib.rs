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

pub use ask::{AskError, Asked, Replied, ask_for_remarks, ask_for_reply};
pub use budget::{Budget, BudgetLevel, BudgetState, month_of};
pub use client::{
    BoxFuture, ChatMessage, LlmClient, LlmError, LlmRequest, LlmResponse, Role, Tokens,
};
pub use config::{
    ApiKey, BudgetConfig, ConfigError, Effort, LlmConfig, Settings, load_api_key, write_api_key,
    write_settings,
};
pub use fake::FakeLlmClient;
pub use openrouter::{ModelInfo, OpenRouterClient, check_key, fetch_models};
pub use validate::{
    MIN_CONFIDENCE, RemarkDraft, RemarkKind, RemarkLevel, ReplyDraft, Rules, ValidRemark,
    parse_remarks, parse_replies, remarks_schema, reply_schema, validate, validate_replies,
};
