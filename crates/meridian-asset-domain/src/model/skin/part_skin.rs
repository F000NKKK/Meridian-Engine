//! How one part's vertices follow a skeleton.

/// The binding of a part to a skeleton. A skinned part keeps its vertices in
/// the skin's own space: the transform of its node is ignored (glTF), so
/// `positions` and `normals` are as authored.
#[derive(Debug, Clone, PartialEq)]
pub struct PartSkin {
    /// Index into [`ModelAsset::skeletons`](crate::ModelAsset::skeletons).
    pub skeleton: usize,
    /// Per vertex, the four joints that move it: indices into the
    /// skeleton's joints.
    pub joints: Vec<[u16; 4]>,
    /// Per vertex, the weight of each of the four joints; they sum to one
    /// (a vertex with no weight is bound fully to its first joint).
    pub weights: Vec<[f32; 4]>,
}
