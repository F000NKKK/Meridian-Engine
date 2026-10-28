//! One blend shape.

/// How one morph target moves a part's vertices at full weight.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MorphTarget {
    /// Per vertex, the offset added to the position.
    pub positions: Vec<[f32; 3]>,
    /// Per vertex, the offset added to the normal before it is renormalised,
    /// or empty when the target leaves normals alone.
    pub normals: Vec<[f32; 3]>,
}
