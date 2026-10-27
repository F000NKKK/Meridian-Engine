//! Shader and compute pipeline creation on the device.

use crate::ComputePipeline;
use crate::Device;
use crate::Shader;

impl Device {
    /// Compiles a WGSL shader module. `label` is for GPU-debugger/error
    /// message purposes only.
    pub fn create_shader(&self, label: &str, wgsl_source: &str) -> Shader {
        let raw = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
            });
        Shader { raw }
    }

    /// Builds a compute pipeline from a single entry point in `shader`.
    /// The bind group layout is auto-derived from the shader's own
    /// `@group`/`@binding` declarations (`layout: None` in `wgpu` terms)
    /// rather than hand-specified — correct and simple for the single
    /// binding-per-dispatch shape [`CommandBuffer::dispatch_compute`](crate::CommandBuffer::dispatch_compute)
    /// supports today; an explicit layout becomes worth building once a
    /// pipeline needs more than one bound resource.
    pub fn create_compute_pipeline(&self, shader: &Shader, entry_point: &str) -> ComputePipeline {
        let raw = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: None,
                module: &shader.raw,
                entry_point: Some(entry_point),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        ComputePipeline { raw }
    }
}
