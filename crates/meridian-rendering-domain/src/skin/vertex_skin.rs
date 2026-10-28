//! How a mesh's vertices follow a skeleton.

/// Per-vertex skinning data of a mesh (ADR 047): the four joints that move
/// each vertex and their weights. A joint is a position in the
/// [`JointPalette`](crate::JointPalette) of the renderable that draws the
/// mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct VertexSkin {
    /// Per vertex, its four joints.
    pub joints: Vec<[u16; 4]>,
    /// Per vertex, the weight of each of its four joints; they sum to one.
    pub weights: Vec<[f32; 4]>,
}
