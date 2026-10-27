//! The registries a scene's handles resolve against.

use super::{MaterialRegistry, MeshRegistry, TextureRegistry};

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
