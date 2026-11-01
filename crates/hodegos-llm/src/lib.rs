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
mod project;
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
pub use project::{
    About, AskedProject, MAX_PROJECT_QUOTES, MAX_REASON_CHARS, ProjectRemarkDraft, ProjectRules,
    ProjectValidated, QuoteDraft, Reevaluated, ReevaluationDraft, Verdict, ask_for_project_remarks,
    ask_for_reevaluation, parse_project_remarks, parse_reevaluation, project_remarks_schema,
    reevaluation_schema, validate_project, validate_reevaluation,
};
pub use validate::{
    MAX_SEE_ALSO, MIN_CONFIDENCE, RemarkDraft, RemarkKind, RemarkLevel, ReplyDraft, Rules,
    SeeAlsoDraft, SeeAlsoSource, ValidRemark, filter_see_also, parse_remarks, parse_replies,
    remarks_schema, remarks_schema_with_see_also, reply_schema, validate, validate_replies,
};
