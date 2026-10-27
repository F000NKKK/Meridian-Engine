//! Vertex attribute formats.

/// A vertex attribute's component format. Not an exhaustive mirror of
/// `wgpu::VertexFormat` (which has ~30 variants for formats this
/// workspace has no use for yet) — extending this is additive as a
/// concrete need shows up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexFormat {
    Float32x2,
    Float32x3,
    Float32x4,
}

impl VertexFormat {
    pub(crate) fn to_wgpu(self) -> wgpu::VertexFormat {
        match self {
            VertexFormat::Float32x2 => wgpu::VertexFormat::Float32x2,
            VertexFormat::Float32x3 => wgpu::VertexFormat::Float32x3,
            VertexFormat::Float32x4 => wgpu::VertexFormat::Float32x4,
        }
    }
}
