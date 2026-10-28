//! Stage trait, identity, scheduler entry and the built-in stage schedules.

mod stage;
mod stage_entry;
mod stage_id;
mod world;

pub use stage::Stage;
pub(crate) use stage_entry::StageEntry;
pub use stage_id::StageId;
pub use world::{WorldStages, add_world_stages};
