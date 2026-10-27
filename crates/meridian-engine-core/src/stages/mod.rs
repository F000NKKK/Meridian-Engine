//! Stage trait, identity and scheduler entry.

mod stage;
mod stage_entry;
mod stage_id;

pub use stage::Stage;
pub(crate) use stage_entry::StageEntry;
pub use stage_id::StageId;
