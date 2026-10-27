//! What a buffer is for.

/// How a [`Buffer`](crate::Buffer) is meant to be bound. Maps to `wgpu::BufferUsages`;
/// every buffer also always gets `COPY_SRC | COPY_DST` — see
/// [`Device::create_buffer`](crate::Device::create_buffer)'s doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsage {
    Vertex,
    Index,
    Uniform,
    Storage,
}

impl BufferUsage {
    pub(crate) fn to_wgpu(self) -> wgpu::BufferUsages {
        match self {
            BufferUsage::Vertex => wgpu::BufferUsages::VERTEX,
            BufferUsage::Index => wgpu::BufferUsages::INDEX,
            BufferUsage::Uniform => wgpu::BufferUsages::UNIFORM,
            BufferUsage::Storage => wgpu::BufferUsages::STORAGE,
        }
    }
}
