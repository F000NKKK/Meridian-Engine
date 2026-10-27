//! The depth-only pipelines and bind groups of the shadow pass.

use crate::submission::SHADOW_DEPTH_SHADER_WGSL;
use crate::submission::shadow_from_colored_layout;
use crate::submission::shadow_from_textured_layout;
use meridian_graphics_driver::{BindGroup, Buffer, BufferUsage, Device, RenderPipeline};

/// What [`SceneRenderer`](crate::submission::SceneRenderer) needs to record the
/// shadow-casters' depth pass: one pipeline per vertex family, each with
/// its own bind group over the shared `light_view_proj` uniform.
///
/// Two bind groups, not one: `wgpu`'s auto-derived (`layout: None`)
/// pipeline layout is unique *per pipeline object*, even for two
/// pipelines built from identical shader text — a bind group built
/// against one pipeline's layout fails validation against the other's, so
/// each pipeline needs its own, both wrapping the same underlying buffer.
#[derive(Debug)]
pub(crate) struct ShadowDepthPass {
    pub(crate) uniform_buffer: Buffer,
    pub(crate) colored_bind_group: BindGroup,
    pub(crate) textured_bind_group: BindGroup,
    pub(crate) colored_pipeline: RenderPipeline,
    pub(crate) textured_pipeline: RenderPipeline,
}

impl ShadowDepthPass {
    /// Builds the pipelines and the `light_view_proj` uniform they read.
    pub(crate) fn new(device: &Device) -> Self {
        let shader = device.create_shader("meridian-shadow-depth", SHADOW_DEPTH_SHADER_WGSL);
        let colored_pipeline =
            device.create_shadow_pipeline(&shader, "vs_main", &shadow_from_colored_layout());
        let textured_pipeline =
            device.create_shadow_pipeline(&shader, "vs_main", &shadow_from_textured_layout());
        let uniform_buffer = device.create_buffer(64, BufferUsage::Uniform);
        // Just the one `light_view_proj` uniform each (see
        // `SHADOW_DEPTH_SHADER_WGSL`'s single `@binding(0)`) — not
        // `create_shadow_bind_group` (that builds the *lit* shaders'
        // 3-binding uniform+shadow-map+shadow-sampler shape; the depth
        // pass itself never samples the shadow map, only writes it).
        let colored_bind_group =
            device.create_uniform_bind_group(&colored_pipeline, &uniform_buffer);
        let textured_bind_group =
            device.create_uniform_bind_group(&textured_pipeline, &uniform_buffer);
        Self {
            uniform_buffer,
            colored_bind_group,
            textured_bind_group,
            colored_pipeline,
            textured_pipeline,
        }
    }
}
