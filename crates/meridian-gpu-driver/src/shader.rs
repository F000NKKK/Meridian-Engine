//! A compiled shader module.

/// A compiled shader module.
#[derive(Debug)]
pub struct Shader {
    pub(crate) raw: wgpu::ShaderModule,
}

impl Shader {
    /// The raw `wgpu::ShaderModule` — needed by `graphics-driver`'s
    /// render pipeline creation (vertex + fragment stages from the same
    /// module), which this crate's own [`Device::create_compute_pipeline`](crate::Device::create_compute_pipeline)
    /// doesn't cover.
    pub fn wgpu_shader(&self) -> &wgpu::ShaderModule {
        &self.raw
    }
}
