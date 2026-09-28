#[macro_use]
mod macros;

pub mod clock;
pub mod error;
pub mod event;
pub mod ids;
pub mod sigils;
pub mod slug;
pub mod status;

pub use error::{MajstackError, Result};
pub use event::{Event, EventKind};
pub use sigils::Sigils;
pub use status::{
    ArtifactType, FailureClass, PermissionLevel, ProviderKind, ReviewVerdict, RunState, Severity,
    TaskStatus, VerificationStatus, WorkType,
};
