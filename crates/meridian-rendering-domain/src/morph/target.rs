//! One blend shape of a mesh.

/// The offsets one morph target adds to the mesh at weight one.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MorphTarget {
    /// Per vertex, the offset added to its position.
    pub positions: Vec<[f32; 3]>,
    /// Per vertex, the offset added to its normal before it is renormalised;
    /// empty when the target leaves normals alone.
    pub normals: Vec<[f32; 3]>,
}
