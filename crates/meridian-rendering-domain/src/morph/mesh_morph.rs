//! The blend shapes of a mesh.

use super::MorphTarget;

/// The morph targets of a mesh. A renderable blends them with its
/// `morph_weights`, one weight per target; the mesh's own positions and
/// normals are what weight zero shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeshMorph {
    /// The targets, each with one offset per vertex.
    pub targets: Vec<MorphTarget>,
}
