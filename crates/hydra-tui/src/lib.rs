//! `hydra-tui` — The cockpit. Hydra's primary interface.

pub mod config;
pub mod constants;
pub mod dot;
pub mod errors;
pub mod input;
pub mod input_search;
pub mod render_markdown;
pub mod stream;
pub mod stream_types;
pub mod theme;
pub mod v2;
pub mod verb;

pub use constants::{ALL_DOT_COLORS, ALL_VERB_COLORS};
pub use dot::DotKind;
pub use errors::TuiError;
pub use input::InputBox;
pub use stream::ConversationStream;
pub use stream_types::{BriefingPriority, CompanionStatus, StreamItem};
pub use verb::{ThinkingVerbState, VerbContext};
