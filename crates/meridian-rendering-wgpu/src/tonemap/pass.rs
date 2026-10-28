//! The HDR to swapchain resolve.

use meridian_rendering_domain::TonemapConfig;

use super::tonemap_shader_wgsl;
use crate::{
    BufferUsage, CommandBuffer, Device, RenderPipeline, Sampler, Surface, SurfaceFrame, Texture,
};

/// The `TonemapUniforms` bytes the shader reads: exposure, operator index,
/// two unused floats.
pub(crate) fn uniform_bytes(config: TonemapConfig) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0..4].copy_from_slice(&config.exposure.to_le_bytes());
    bytes[4..8].copy_from_slice(&config.operator.shader_index().to_le_bytes());
    bytes
}

/// Resolves an HDR scene texture into a swapchain frame: exposure, then the
/// configured tone curve. Owns one fullscreen pipeline built for the surface's
/// format.
#[derive(Debug)]
pub struct TonemapPass {
    pipeline: RenderPipeline,
    sampler: Sampler,
    /// Exposure and curve, changeable between frames.
    pub config: TonemapConfig,
}

impl TonemapPass {
    /// A pass writing to frames of `surface`.
    #[must_use]
    pub fn new(device: &Device, surface: &Surface) -> Self {
        let shader = device.create_shader("meridian-tonemap", &tonemap_shader_wgsl());
        Self {
            pipeline: device.create_fullscreen_pipeline(&shader, "fs_main", surface, false),
            sampler: device.create_clamp_sampler(),
            config: TonemapConfig::default(),
        }
    }

    /// Records the resolve of `scene` into `frame`, replacing its contents.
    pub fn apply(
        &self,
        device: &Device,
        commands: &mut CommandBuffer<'_>,
        scene: &Texture,
        frame: &SurfaceFrame,
    ) {
        let uniform = device.create_buffer(16, BufferUsage::Uniform);
        device.write_buffer(&uniform, &uniform_bytes(self.config));
        let bind_group =
            device.create_textured_bind_group(&self.pipeline, &uniform, scene, &self.sampler);
        let mut pass = commands.begin_render_pass(frame.view(), [0.0, 0.0, 0.0, 1.0], None);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group);
        pass.draw(0..3);
    }
}
