//! A resource bind group.

/// A bound resource group — today, always the single-buffer shape
/// [`Device::create_single_buffer_bind_group`](crate::Device::create_single_buffer_bind_group) builds.
#[derive(Debug)]
pub struct BindGroup {
    pub(crate) raw: wgpu::BindGroup,
}

impl BindGroup {
    pub fn wgpu_bind_group(&self) -> &wgpu::BindGroup {
        &self.raw
    }
}
