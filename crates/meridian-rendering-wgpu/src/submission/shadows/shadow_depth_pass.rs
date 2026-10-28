//! The depth-only pipeline and bind group of the shadow pass.

use crate::submission::{
    SHADOW_DEPTH_SHADER_WGSL, position_only_mesh_layout, shadow_instance_layout,
};
use crate::{BindGroup, Buffer, BufferUsage, Device, RenderPipeline};

/// What [`SceneRenderer`](crate::submission::SceneRenderer) needs to record
/// the shadow-casters' depth pass: one pipeline over the mesh and instance
/// buffers every batch shares, with a bind group over the `light_view_proj`
/// uniform.
#[derive(Debug)]
pub(crate) struct ShadowDepthPass {
    pub(crate) uniform_buffer: Buffer,
    pub(crate) bind_group: BindGroup,
    pub(crate) pipeline: RenderPipeline,
}

impl ShadowDepthPass {
    /// Builds the pipeline and the `light_view_proj` uniform it reads.
    pub(crate) fn new(device: &Device) -> Self {
        let shader = device.create_shader("meridian-shadow-depth", SHADOW_DEPTH_SHADER_WGSL);
        let pipeline = device.create_shadow_pipeline(
            &shader,
            "vs_main",
            &[position_only_mesh_layout(), shadow_instance_layout()],
        );
        let uniform_buffer = device.create_buffer(64, BufferUsage::Uniform);
        // Just the one `light_view_proj` uniform (see
        // `SHADOW_DEPTH_SHADER_WGSL`'s single `@binding(0)`): the depth
        // pass writes the shadow map, it never samples it.
        let bind_group = device.create_uniform_bind_group(&pipeline, &uniform_buffer);
        Self {
            uniform_buffer,
            bind_group,
            pipeline,
        }
    }
}
