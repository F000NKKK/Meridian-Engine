//! Everything a recorder needs to draw one frame.

use crate::scene::Scene3d;
use crate::{MaterialRegistry, MeshRegistry};

/// One frame to draw: the scene, the registries its handles resolve against,
/// and the colour the target is cleared to.
#[derive(Debug, Clone, Copy)]
pub struct FrameRequest<'a> {
    /// The culled scene to draw.
    pub scene: &'a Scene3d,
    /// Resolves the scene's mesh handles.
    pub meshes: &'a MeshRegistry,
    /// Resolves the scene's material handles.
    pub materials: &'a MaterialRegistry,
    /// Clear colour, RGBA.
    pub clear_color: [f64; 4],
}
