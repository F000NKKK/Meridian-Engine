//! One drawable piece of a loaded model.

use meridian_rendering_domain::{Material, MeshHandle};

/// One drawable piece of a loaded model: the mesh to draw and the material
/// to draw it with. Its geometry is already in the model's own space, so a
/// renderable places it with the frame of the whole model.
#[derive(Debug, Clone, Copy)]
pub struct ModelPiece {
    /// The registered geometry.
    pub mesh: MeshHandle,
    /// The surface, with its texture already uploaded.
    pub material: Material,
    /// The skeleton (an index into [`ModelRig::skeletons`](crate::assets::ModelRig::skeletons))
    /// that moves the mesh, for a skinned piece. Its palette goes in the
    /// renderable's `palette`, and the renderable's `frame` places the whole
    /// model.
    pub skeleton: Option<usize>,
}
