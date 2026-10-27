//! Where an entity lives: archetype and row.

#[derive(Debug, Clone, Copy)]
pub(crate) struct EntityLocation {
    pub(crate) archetype: usize,
    pub(crate) row: usize,
}
