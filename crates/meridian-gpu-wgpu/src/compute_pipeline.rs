//! A compiled compute pipeline.

/// A configured compute pipeline (shader + bind group layout).
#[derive(Debug)]
pub struct ComputePipeline {
    pub(crate) raw: wgpu::ComputePipeline,
}

impl ComputePipeline {
    /// The bind group layout `Device::create_compute_pipeline` derived
    /// from the shader — an escape hatch for callers building their own
    /// bind groups beyond [`Device::create_single_buffer_bind_group`](crate::Device::create_single_buffer_bind_group)'s
    /// single-binding shape.
    #[must_use]
    pub fn bind_group_layout(&self) -> wgpu::BindGroupLayout {
        self.raw.get_bind_group_layout(0)
    }

    /// The raw `wgpu::ComputePipeline` — an escape hatch for callers
    /// recording their own compute pass with a bind group shape beyond
    /// [`Device::create_single_buffer_bind_group`](crate::Device::create_single_buffer_bind_group)'s single-binding one
    /// (see [`CommandBuffer::encoder_mut`](crate::CommandBuffer::encoder_mut)'s doc comment for the same
    /// pattern applied to command recording).
    #[must_use]
    pub fn wgpu_pipeline(&self) -> &wgpu::ComputePipeline {
        &self.raw
    }
}
