//! Entity identity: an index plus a generation.

use meridian_memory_core::Handle;

/// An opaque, generational entity id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}

impl From<Handle> for Entity {
    fn from(h: Handle) -> Self {
        Entity {
            index: h.index,
            generation: h.generation,
        }
    }
}

impl From<Entity> for Handle {
    fn from(e: Entity) -> Self {
        Handle {
            index: e.index,
            generation: e.generation,
        }
    }
}
