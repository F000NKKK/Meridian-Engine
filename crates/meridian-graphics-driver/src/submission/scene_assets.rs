//! The registries a scene's handles resolve against.

use meridian_graphics_core::{MaterialRegistry, MeshRegistry};

use super::TextureRegistry;

/// The mesh, material and texture registries that resolve a scene's
/// handles while it is baked for drawing.
#[derive(Debug, Clone, Copy)]
pub struct SceneAssets<'a> {
    /// Registered meshes.
    pub meshes: &'a MeshRegistry,
    /// Registered materials.
    pub materials: &'a MaterialRegistry,
    /// Registered textures.
    pub textures: &'a TextureRegistry,
}
