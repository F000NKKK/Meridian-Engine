//! Everything a recorder needs to draw one frame.

use meridian_rendering_domain::{MaterialRegistry, MeshRegistry, Overlay2d, Scene3d};

/// One frame to draw: the scene, the registries its handles resolve against,
/// the overlays drawn over it, and the colour the target is cleared to.
#[derive(Debug, Clone, Copy)]
pub struct FrameRequest<'a> {
    /// The culled scene to draw.
    pub scene: &'a Scene3d,
    /// Resolves the scene's mesh handles.
    pub meshes: &'a MeshRegistry,
    /// Resolves the scene's material handles.
    pub materials: &'a MaterialRegistry,
    /// 2D scenes drawn over the 3D frame, unblurred, in slice order.
    pub overlays: &'a [Overlay2d],
    /// Independent 3D views drawn in their pixel rectangles before the 2D overlays.
    pub viewports: &'a [Viewport3d],
    /// Clear colour, RGBA.
    pub clear_color: [f64; 4],
}
