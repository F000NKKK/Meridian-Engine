//! The depth-only pipeline and bind group of the shadow pass.

use crate::submission::{
    SHADOW_DEPTH_SHADER_WGSL, SHADOW_TILES, position_only_mesh_layout, shadow_instance_layout,
};
use crate::{BindGroup, Buffer, BufferUsage, Device, RenderPipeline};

/// What [`SceneRenderer`](crate::submission::SceneRenderer) needs to record
/// the shadow-casters' depth pass: one pipeline over the mesh and instance
/// buffers every batch shares, and per tile a `light_view_proj` uniform
/// with the bind group over it.
#[derive(Debug)]
pub(crate) struct ShadowDepthPass {
    pub(crate) tiles: [TileUniform; SHADOW_TILES],
    pub(crate) pipeline: RenderPipeline,
}

/// One tile's `light_view_proj` uniform and its bind group.
#[derive(Debug)]
pub(crate) struct TileUniform {
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
        // Just the one `light_view_proj` uniform per tile (see
        // `SHADOW_DEPTH_SHADER_WGSL`'s single `@binding(0)`): the depth
        // pass writes the shadow map, it never samples it.
        let tiles = std::array::from_fn(|_| {
            let buffer = device.create_buffer(64, BufferUsage::Uniform);
            let bind_group = device.create_uniform_bind_group(&pipeline, &buffer);
            TileUniform { buffer, bind_group }
        });
        Self { tiles, pipeline }
    }
}
