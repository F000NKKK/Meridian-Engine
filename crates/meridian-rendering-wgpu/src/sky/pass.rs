//! Draws the environment behind the scene.

use meridian_gac::Vec3;
use meridian_rendering_domain::{Background, Camera, Scene3d};

use super::sky_shader_wgsl;
use crate::submission::GpuEnvironment;
use crate::{BufferUsage, ColorFormat, Device, RenderPass, RenderPipeline};

/// The `SkyUniforms` bytes: the camera's world axes, then the half-extents of
/// its view plane and the mip level to read. `None` for a camera without a
/// perspective projection, which has no view rays to look up.
pub(crate) fn uniform_bytes(camera: &Camera, lod: f32) -> Option<[u8; 64]> {
    let m = camera.projection.0;
    let perspective = (m[2][3] + 1.0).abs() < 1e-6 && m[0][0] > 0.0 && m[1][1] > 0.0;
    if !perspective {
        return None;
    }
    // The camera frame looks along +X with +Y up and +Z to the right.
    let axes = [Vec3::Z, Vec3::Y, Vec3::X].map(|axis| camera.frame.transform_vector(axis));
    let mut bytes = [0u8; 64];
    let mut put = |index: usize, value: f32| {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    };
    for (row, axis) in axes.iter().enumerate() {
        put(row * 4, axis.x);
        put(row * 4 + 1, axis.y);
        put(row * 4 + 2, axis.z);
    }
    put(12, 1.0 / m[0][0]);
    put(13, 1.0 / m[1][1]);
    put(14, lod);
    Some(bytes)
}

/// Draws a scene's [`Background::Environment`] as the first thing of the main
/// pass. Owns one pipeline built for the scene target's format.
#[derive(Debug)]
pub struct SkyPass {
    pipeline: RenderPipeline,
}

impl SkyPass {
    /// A pass drawing into targets of `format`.
    #[must_use]
    pub fn new(device: &Device, format: ColorFormat) -> Self {
        let shader = device.create_shader("meridian-sky", &sky_shader_wgsl());
        Self {
            pipeline: device.create_sky_pipeline(&shader, "fs_main", format),
        }
    }

    /// Records the backdrop into `pass` when the scene's background asks for the
    /// environment and `environment` holds one; otherwise records nothing, and
    /// the pass keeps its clear colour.
    pub(crate) fn draw(
        &self,
        device: &Device,
        pass: &mut RenderPass<'_>,
        scene: &Scene3d,
        environment: &GpuEnvironment,
    ) {
        let Background::Environment { blur } = scene.background else {
            return;
        };
        if !environment.uniform.enabled() {
            return;
        }
        let lod = blur.clamp(0.0, 1.0) * environment.uniform.max_lod();
        let Some(bytes) = uniform_bytes(&scene.camera, lod) else {
            return;
        };
        let uniform = device.create_buffer(bytes.len(), BufferUsage::Uniform);
        device.write_buffer(&uniform, &bytes);
        let bind_group = device.create_textured_bind_group(
            &self.pipeline,
            &uniform,
            &environment.texture,
            &environment.sampler,
        );
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group);
        pass.draw(0..3);
    }
}
