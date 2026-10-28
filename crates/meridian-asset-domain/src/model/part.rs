//! One drawable piece of a model.

use super::ModelMaterial;

/// One triangle-list primitive with its node transforms already applied, so
/// every part is in the model's own space and carries no matrix of its own.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelPart {
    /// Vertex positions in the model's space.
    pub positions: Vec<[f32; 3]>,
    /// One unit normal per position, or empty when the source has none.
    pub normals: Vec<[f32; 3]>,
    /// One UV per position, or empty when the source has none.
    pub uvs: Vec<[f32; 2]>,
    /// Triangle list, every index below `positions.len()`.
    pub indices: Vec<u32>,
    /// How the part is shaded.
    pub material: ModelMaterial,
}
