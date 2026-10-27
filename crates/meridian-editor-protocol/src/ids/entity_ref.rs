//! [`EntityRef`]: which entity of the running game a message is about.

use meridian_ecs_core::Entity;
use substrate_data::Schema;

/// An entity of the running game: its `ecs-core` index and generation. Valid
/// for one run of the host — a restart or a scene reload may hand the same
/// numbers to other entities — so an editor re-lists entities after
/// reconnecting. A despawned entity's generation never matches again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Schema)]
pub struct EntityRef {
    /// The entity's slot.
    pub index: u32,
    /// How many times the slot has been reused.
    pub generation: u32,
}

impl From<Entity> for EntityRef {
    fn from(entity: Entity) -> Self {
        EntityRef {
            index: entity.index,
            generation: entity.generation,
        }
    }
}

impl From<EntityRef> for Entity {
    fn from(entity: EntityRef) -> Self {
        Entity {
            index: entity.index,
            generation: entity.generation,
        }
    }
}
