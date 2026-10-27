//! Bind group creation on the device.

use crate::{BindGroup, Buffer, Device, RenderPipeline, SampledTexture, Sampler, Texture};

impl Device {
    /// Builds a bind group binding `buffer` (a uniform, as in
    /// [`create_uniform_bind_group`](Self::create_uniform_bind_group)) at
    /// binding 0, `texture`'s view at binding 1 and `sampler` at binding
    /// 2 of `pipeline`'s auto-derived layout — the shape a shader that
    /// samples one albedo texture alongside its view-projection uniform
    /// needs.
    pub fn create_textured_bind_group(
        &self,
        pipeline: &RenderPipeline,
        buffer: &Buffer,
        texture: &Texture,
        sampler: &Sampler,
    ) -> BindGroup {
        self.0.create_texture_bind_group(
            &pipeline.raw.get_bind_group_layout(0),
            buffer,
            texture,
            sampler,
        )
    }

    /// Like [`create_textured_bind_group`](Self::create_textured_bind_group)
    /// plus a shadow map and its comparison sampler — the bind group a
    /// textured, shadow-receiving lit pipeline needs (see
    /// `graphics-core::submission`'s `lit_textured_shadow_shader_wgsl`
    /// for the binding declarations this must match).
    pub fn create_textured_shadow_bind_group(
        &self,
        pipeline: &RenderPipeline,
        buffer: &Buffer,
        albedo: SampledTexture<'_>,
        shadow: SampledTexture<'_>,
    ) -> BindGroup {
        self.0.create_textured_shadow_bind_group(
            &pipeline.raw.get_bind_group_layout(0),
            buffer,
            albedo,
            shadow,
        )
    }

    /// Like [`create_uniform_bind_group`](Self::create_uniform_bind_group)
    /// plus a shadow map and its comparison sampler — the bind group an
    /// untextured (vertex-colored), shadow-receiving lit pipeline needs.
    pub fn create_shadow_bind_group(
        &self,
        pipeline: &RenderPipeline,
        buffer: &Buffer,
        shadow_map: &Texture,
        shadow_sampler: &Sampler,
    ) -> BindGroup {
        self.0.create_shadow_bind_group(
            &pipeline.raw.get_bind_group_layout(0),
            buffer,
            shadow_map,
            shadow_sampler,
        )
    }

    /// Builds a bind group binding `buffer` (typically a `Uniform`
    /// buffer — a per-frame MVP matrix, for instance) at `@group(0)
    /// @binding(0)` of `pipeline` — the same single-binding shape
    /// [`Device::create_render_pipeline`](crate::Device::create_render_pipeline)'s auto-derived layout expects.
    pub fn create_uniform_bind_group(
        &self,
        pipeline: &RenderPipeline,
        buffer: &meridian_gpu_driver::Buffer,
    ) -> meridian_gpu_driver::BindGroup {
        self.0
            .create_single_buffer_bind_group(&pipeline.raw.get_bind_group_layout(0), buffer)
    }
}
