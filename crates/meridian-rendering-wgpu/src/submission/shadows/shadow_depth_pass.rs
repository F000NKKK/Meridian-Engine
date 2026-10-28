//! The depth-only pipeline and bind group of the shadow pass.

use crate::submission::{
    SHADOW_DEPTH_SHADER_WGSL, position_only_mesh_layout, shadow_instance_layout,
};
use meridian_rendering_domain::SHADOW_CASCADES;

use crate::{BindGroup, Buffer, BufferUsage, Device, RenderPipeline};

/// What [`SceneRenderer`](crate::submission::SceneRenderer) needs to record
/// the shadow-casters' depth pass: one pipeline over the mesh and instance
/// buffers every batch shares, and per cascade a `light_view_proj` uniform
/// with the bind group over it.
#[derive(Debug)]
pub(crate) struct ShadowDepthPass {
    pub(crate) cascades: [CascadeUniform; SHADOW_CASCADES],
    pub(crate) pipeline: RenderPipeline,
}

/// One cascade's `light_view_proj` uniform and its bind group.
#[derive(Debug)]
pub(crate) struct CascadeUniform {
    pub(crate) buffer: Buffer,
    pub(crate) bind_group: BindGroup,
}

impl ShadowDepthPass {
    /// Builds the pipeline and the `light_view_proj` uniforms it reads.
    pub(crate) fn new(device: &Device) -> Self {
        let shader = device.create_shader("meridian-shadow-depth", SHADOW_DEPTH_SHADER_WGSL);
        let pipeline = device.create_shadow_pipeline(
            &shader,
            "vs_main",
            &[position_only_mesh_layout(), shadow_instance_layout()],
        );
        // Just the one `light_view_proj` uniform per cascade (see
        // `SHADOW_DEPTH_SHADER_WGSL`'s single `@binding(0)`): the depth
        // pass writes the shadow map, it never samples it.
        let cascades = std::array::from_fn(|_| {
            let buffer = device.create_buffer(64, BufferUsage::Uniform);
            let bind_group = device.create_uniform_bind_group(&pipeline, &buffer);
            CascadeUniform { buffer, bind_group }
        });
        Self { cascades, pipeline }
    }
}
