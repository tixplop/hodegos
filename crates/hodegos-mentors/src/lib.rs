//! Registre des mentors, casting, paliers et construction des prompts.
//!
//! Dépend de `hodegos-core`. Ne connaît ni HTTP ni la WebSocket.

mod casting;
mod eject;
mod mentor;
mod project_prompt;
mod prompt;
mod registry;
mod section_prompt;
mod watch;

pub use casting::{
    CastRequest, Casting, Forced, ForcedBy, cast, frontmatter_mentor, section_headings,
};
pub use eject::{EjectError, eject};
pub use mentor::{
    ContextScope, FieldError, FormTriggers, Level, Mentor, MentorError, MentorForm, Mode,
    ModelTier, Origin, Parsed, ProjectSettings, ReviewTrigger, Scope, Triggers, join,
    minimal_override, parse, read_form, render, validate,
};
pub use project_prompt::{
    FileDiff, FileText, OpenRemark, ProjectFollowUpInput, ReevaluationInput, ReviewInput,
    project_follow_up_prompt, reevaluation_prompt, review_prompt,
};
pub use prompt::{
    Author, Extra, FollowUp, FollowUpKind, NeighborText, Prompt, PromptInput, build_prompt,
    system_prompt,
};
pub use registry::{MentorFile, MentorSources, Registry, Warning, default_user_dir, inventory};
pub use section_prompt::{CONTEXT_LINES, SectionRequest, Thread, section_prompt};
pub use watch::{MentorWatcher, WatchError};
