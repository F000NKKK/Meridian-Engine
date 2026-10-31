//! Decoded mesh geometry.

/// Decoded mesh data (vertices/indices), not yet uploaded to the GPU.
/// Empty normals or UVs ask the consumer to derive their defaults.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshAsset {
    /// Vertex positions in model space.
    pub positions: Vec<[f32; 3]>,
    /// Per-vertex normals, or empty when the consumer should derive them.
    pub normals: Vec<[f32; 3]>,
    /// Per-vertex texture coordinates, or empty when unavailable.
    pub uvs: Vec<[f32; 2]>,
    /// Triangle indices into the vertex arrays.
    pub indices: Vec<u32>,
}
