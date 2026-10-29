//! Registre des mentors, casting, paliers et construction des prompts.
//!
//! Dépend de `hodegos-core`. Ne connaît ni HTTP ni la WebSocket.

mod casting;
mod eject;
mod mentor;
mod prompt;
mod registry;
mod watch;

pub use casting::{CastRequest, Casting, Forced, ForcedBy, cast, frontmatter_mentor};
pub use eject::{EjectError, eject};
pub use mentor::{Level, Mentor, MentorError, Mode, ModelTier, Origin, Parsed, Triggers, parse};
pub use prompt::{Prompt, PromptInput, build_prompt};
pub use registry::{MentorSources, Registry, Warning, default_user_dir};
pub use watch::{MentorWatcher, WatchError};
