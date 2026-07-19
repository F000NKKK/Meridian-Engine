//! Frame/persistent arenas, resource pools and generational handles; minimizes dynamic allocation across the engine.

use core::marker::PhantomData;

/// A generational index into a resource pool slot. Plain, `Copy`,
/// serializable — not a smart pointer. See docs/memory-model.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Handle {
    pub index: u32,
    pub generation: u32,
}

/// Bump allocator reset (not freed-and-reallocated) at the end of each frame.
pub struct FrameArena;

/// Bump allocator freed in bulk when its owning scope ends.
pub struct PersistentArena;

/// Handle-addressed storage for resources with irregular, individually
/// tracked lifetimes.
pub struct ResourcePool<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for ResourcePool<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
