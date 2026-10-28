//! A GPU buffer.

/// A GPU-visible buffer (vertex/index/uniform/storage).
#[derive(Debug)]
pub struct Buffer {
    pub(crate) raw: wgpu::Buffer,
    /// Size of the buffer in bytes, as passed to
    /// [`Device::create_buffer`](crate::Device::create_buffer).
    pub byte_len: usize,
}

impl Buffer {
    /// The raw `wgpu::Buffer` — an escape hatch for `rendering-wgpu`'s
    /// own additional buffer operations (setting it as a vertex/index
    /// buffer on a render pass) this crate doesn't know about.
    #[must_use]
    pub fn wgpu_buffer(&self) -> &wgpu::Buffer {
        &self.raw
    }
}
