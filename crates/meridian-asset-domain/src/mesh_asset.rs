//! Decoded mesh geometry.

/// Decoded mesh data (vertices/indices), not yet uploaded to the GPU.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshAsset {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}
