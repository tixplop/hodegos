//! Cœur de Hodegos : projet, fichiers, diffs, sections et contexte de session.
//!
//! Ne dépend d'aucun autre crate de Hodegos et ne connaît ni HTTP ni la WebSocket.

mod error;
mod files;
mod ignore_rules;
mod language;
mod paths;
mod project;
mod sections;
mod session;
mod tree;
mod watch;

pub use error::FileError;
pub use files::{MAX_TEXT_BYTES, TextFile, Version};
pub use language::Language;
pub use paths::RelPath;
pub use project::{Project, ProjectError};
pub use sections::{Section, frontmatter, section_at, sections};
pub use session::{ProjectId, Session, UserId};
pub use tree::{DirEntry, EntryKind};
pub use watch::FileWatcher;
