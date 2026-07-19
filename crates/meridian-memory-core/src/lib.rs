//! Frame/persistent arenas, resource pools and generational handles; minimizes dynamic allocation across the engine.
//!
//! See docs/memory-model.md for the three allocation strategies this crate
//! covers and why handles replace `Arc<T>` for resource lifetime.

/// A generational index into a resource pool slot. Plain, `Copy`,
/// serializable — not a smart pointer. See docs/memory-model.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Handle {
    pub index: u32,
    pub generation: u32,
}

/// Handle-addressed storage for resources with irregular, individually
/// tracked lifetimes: a classic generational slot pool, not reference
/// counting. Looking a [`Handle`] up against a slot whose generation moved
/// on (the slot was freed and possibly reused) returns `None` instead of
/// aliasing unrelated data.
#[derive(Debug)]
pub struct ResourcePool<T> {
    slots: Vec<Option<T>>,
    generations: Vec<u32>,
    free_list: Vec<u32>,
    len: usize,
}

impl<T> Default for ResourcePool<T> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            generations: Vec::new(),
            free_list: Vec::new(),
            len: 0,
        }
    }
}

impl<T> ResourcePool<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Inserts `value`, reusing a freed slot (with a bumped generation) if
    /// one is available, otherwise growing the pool.
    pub fn insert(&mut self, value: T) -> Handle {
        self.len += 1;
        if let Some(index) = self.free_list.pop() {
            let generation = self.generations[index as usize];
            self.slots[index as usize] = Some(value);
            Handle { index, generation }
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Some(value));
            self.generations.push(0);
            Handle {
                index,
                generation: 0,
            }
        }
    }

    fn generation_matches(&self, handle: Handle) -> bool {
        self.generations.get(handle.index as usize) == Some(&handle.generation)
    }

    pub fn contains(&self, handle: Handle) -> bool {
        self.generation_matches(handle) && self.slots[handle.index as usize].is_some()
    }

    pub fn get(&self, handle: Handle) -> Option<&T> {
        if !self.generation_matches(handle) {
            return None;
        }
        self.slots[handle.index as usize].as_ref()
    }

    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
        if !self.generation_matches(handle) {
            return None;
        }
        self.slots[handle.index as usize].as_mut()
    }

    /// Removes and returns the value, bumping the slot's generation so any
    /// other `Handle` still pointing at it stops resolving. Returns `None`
    /// if `handle` doesn't resolve (already removed, or stale).
    pub fn remove(&mut self, handle: Handle) -> Option<T> {
        if !self.generation_matches(handle) {
            return None;
        }
        let value = self.slots[handle.index as usize].take()?;
        self.generations[handle.index as usize] = handle.generation.wrapping_add(1);
        self.free_list.push(handle.index);
        self.len -= 1;
        Some(value)
    }
}

/// A bump-style list: allocation is a monotonic push, addressed by index
/// rather than by a returned reference (so allocating doesn't fight the
/// borrow checker over previously-allocated items). [`reset`](Self::reset)
/// clears the list without releasing its backing capacity — the "reset,
/// not freed-and-reallocated" behavior docs/memory-model.md calls for.
/// `FrameArena<T>` and [`PersistentArena<T>`] share this shape; they're
/// distinct types so a frame-scoped and a level-scoped arena can't be
/// accidentally swapped at a call site — the difference is *when* the
/// owning subsystem calls `reset`, not how the arena behaves.
#[derive(Debug)]
pub struct FrameArena<T> {
    items: Vec<T>,
}

impl<T> Default for FrameArena<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> FrameArena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc(&mut self, value: T) -> usize {
        self.items.push(value);
        self.items.len() - 1
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Clears the arena for reuse next frame. Capacity is retained, so a
    /// frame that allocates roughly the same amount each time settles into
    /// zero further heap growth.
    pub fn reset(&mut self) {
        self.items.clear();
    }
}

/// Same allocation shape as [`FrameArena`], for data that outlives a
/// single frame but still follows a coarse, predictable lifetime
/// (level-lifetime data, loaded-scene data) — freed in bulk via
/// [`reset`](Self::reset) when its owning scope ends, not piecemeal.
#[derive(Debug)]
pub struct PersistentArena<T> {
    items: Vec<T>,
}

impl<T> Default for PersistentArena<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> PersistentArena<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc(&mut self, value: T) -> usize {
        self.items.push(value);
        self.items.len() - 1
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn reset(&mut self) {
        self.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_insert_and_get() {
        let mut pool = ResourcePool::new();
        let h = pool.insert(42);
        assert_eq!(pool.get(h), Some(&42));
        assert_eq!(pool.len(), 1);
        assert!(!pool.is_empty());
    }

    #[test]
    fn pool_get_mut_mutates_in_place() {
        let mut pool = ResourcePool::new();
        let h = pool.insert(1);
        *pool.get_mut(h).unwrap() = 2;
        assert_eq!(pool.get(h), Some(&2));
    }

    #[test]
    fn pool_remove_returns_value_and_empties_slot() {
        let mut pool = ResourcePool::new();
        let h = pool.insert("a");
        assert_eq!(pool.remove(h), Some("a"));
        assert_eq!(pool.get(h), None);
        assert_eq!(pool.remove(h), None);
        assert!(pool.is_empty());
    }

    #[test]
    fn pool_stale_handle_after_reuse_does_not_alias() {
        let mut pool = ResourcePool::new();
        let h1 = pool.insert(1);
        pool.remove(h1).unwrap();

        // Reusing the freed slot must bump the generation, not reuse h1's.
        let h2 = pool.insert(2);
        assert_eq!(h1.index, h2.index, "test assumes the freed slot is reused");
        assert_ne!(h1.generation, h2.generation);

        assert_eq!(
            pool.get(h1),
            None,
            "stale handle must not resolve to the new value"
        );
        assert_eq!(pool.get(h2), Some(&2));
    }

    #[test]
    fn pool_contains_matches_get() {
        let mut pool = ResourcePool::new();
        let h = pool.insert(());
        assert!(pool.contains(h));
        pool.remove(h);
        assert!(!pool.contains(h));
    }

    #[test]
    fn pool_len_tracks_inserts_and_removes_across_reuse() {
        let mut pool = ResourcePool::new();
        let a = pool.insert(1);
        let _b = pool.insert(2);
        assert_eq!(pool.len(), 2);
        pool.remove(a);
        assert_eq!(pool.len(), 1);
        pool.insert(3);
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn frame_arena_alloc_and_reset() {
        let mut arena: FrameArena<u32> = FrameArena::new();
        let i0 = arena.alloc(10);
        let i1 = arena.alloc(20);
        assert_eq!(arena.get(i0), Some(&10));
        assert_eq!(arena.get(i1), Some(&20));
        assert_eq!(arena.len(), 2);

        arena.reset();
        assert!(arena.is_empty());
        assert_eq!(arena.get(i0), None);

        // Capacity survives reset; a fresh frame reuses index 0 again.
        let i0_next_frame = arena.alloc(30);
        assert_eq!(i0_next_frame, 0);
        assert_eq!(arena.get(0), Some(&30));
    }

    #[test]
    fn persistent_arena_alloc_and_reset() {
        let mut arena: PersistentArena<&str> = PersistentArena::new();
        arena.alloc("level data");
        arena.alloc("more level data");
        assert_eq!(arena.len(), 2);
        arena.reset();
        assert!(arena.is_empty());
    }
}
