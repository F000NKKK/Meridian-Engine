//! A resource bind group.

/// A bound resource group — today, always the single-buffer shape
/// [`Device::create_single_buffer_bind_group`](crate::Device::create_single_buffer_bind_group) builds.
#[derive(Debug)]
pub struct BindGroup {
    pub(crate) raw: wgpu::BindGroup,
}

impl BindGroup {
    /// The raw `wgpu::BindGroup`, for callers that set it on a render or
    /// compute pass this crate does not record itself.
    #[must_use]
    pub fn wgpu_bind_group(&self) -> &wgpu::BindGroup {
        &self.raw
    }
}
