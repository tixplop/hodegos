//! Cœur de Hodegos : projet, fichiers, diffs, sections et contexte de session.
//!
//! Ne dépend d'aucun autre crate de Hodegos et ne connaît ni HTTP ni la WebSocket.

mod project;
mod session;

pub use project::{Project, ProjectError};
pub use session::UserId;
