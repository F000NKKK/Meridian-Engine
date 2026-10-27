//! The port that keeps mesh sources addressable by handle.

use meridian_graphics_core::MeshHandle;
use meridian_graphics_core::{MeshRegistryError, MeshSource};

/// Where mesh sources live, addressed by [`MeshHandle`]. A store validates a
/// mesh when it takes it: a triangle list, at most `u16::MAX` vertices, and one
/// normal and one UV per position.
///
/// [`MeshRegistry`](meridian_graphics_core::MeshRegistry) is the in-memory implementation and
/// runs [`testing::run_mesh_store_contract`](crate::testing::run_mesh_store_contract).
pub trait MeshStore {
    /// Validates and stores `source`.
    ///
    /// # Errors
    ///
    /// [`MeshRegistryError`] naming the first broken rule.
    fn add(&mut self, source: MeshSource) -> Result<MeshHandle, MeshRegistryError>;

    /// The source behind `handle`, if it is live.
    fn get(&self, handle: MeshHandle) -> Option<&MeshSource>;

    /// Removes and returns the source behind `handle`.
    fn remove(&mut self, handle: MeshHandle) -> Option<MeshSource>;
}
