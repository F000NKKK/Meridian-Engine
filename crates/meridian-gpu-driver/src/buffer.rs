//! A GPU buffer.

/// A GPU-visible buffer (vertex/index/uniform/storage).
#[derive(Debug)]
pub struct Buffer {
    pub(crate) raw: wgpu::Buffer,
    pub byte_len: usize,
}

impl Buffer {
    /// The raw `wgpu::Buffer` — an escape hatch for `graphics-driver`'s
    /// own additional buffer operations (setting it as a vertex/index
    /// buffer on a render pass) this crate doesn't know about.
    pub fn wgpu_buffer(&self) -> &wgpu::Buffer {
        &self.raw
    }
}
