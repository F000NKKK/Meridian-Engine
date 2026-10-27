//! Stage trait, identity, context and scheduler entry.

mod context;
mod entry;
mod id;
mod stage;

pub use context::StageContext;
pub(crate) use entry::StageEntry;
pub use id::StageId;
pub use stage::Stage;
