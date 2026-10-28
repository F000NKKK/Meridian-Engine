//! One drawable piece of a model.

use super::{ModelMaterial, PartMorph, PartSkin};

/// One triangle-list primitive. A rigid part has its node transforms already
/// applied, so it is in the model's own space and carries no matrix of its
/// own; a skinned part (`skin` is set) stays as authored and follows its
/// skeleton instead.
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
    /// The skeleton the vertices follow, for a skinned part.
    pub skin: Option<PartSkin>,
    /// The blend shapes, when the part has any.
    pub morph: Option<PartMorph>,
}
