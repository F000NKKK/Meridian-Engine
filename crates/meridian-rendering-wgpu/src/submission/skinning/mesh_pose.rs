//! What poses one mesh instance this frame.

use meridian_rendering_domain::JointPalette;

/// The joint matrices and morph weights a renderable gives its mesh.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MeshPose<'a> {
    /// The palette that skins the mesh, if it is skinned.
    pub(crate) palette: Option<&'a JointPalette>,
    /// The weight of each morph target; short means zero past the end.
    pub(crate) weights: &'a [f32],
}
