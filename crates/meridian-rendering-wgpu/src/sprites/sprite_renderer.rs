//! The pipeline and sampler that draw sprite batches.

use meridian_rendering_domain::scene::Camera2d;
use meridian_rendering_domain::{ScissorRect, SpriteBatch, SpriteView};

use super::sprite_encoding::{sprite_vertex_layout, write_vertex};
use super::sprite_run::SpriteRun;
use super::{PreparedSprites, SPRITE_SHADER_WGSL};
use crate::submission::TextureRegistry;
use crate::{
    Buffer, BufferUsage, ColorFormat, Device, RenderPass, RenderPipeline, Sampler, ShaderStages,
    Surface, VertexLayout,
};

/// Draws [`SpriteBatch`]es over whatever the target already holds: no
/// depth, premultiplied-alpha blending, clamp-to-edge linear sampling.
/// View-independent, so one renderer serves every `Scene2d` of a frame.
#[derive(Debug)]
pub struct SpriteRenderer {
    pipeline: RenderPipeline,
    sampler: Sampler,
}

impl SpriteRenderer {
    /// A renderer for passes targeting `surface`'s swapchain.
    #[must_use]
    pub fn new(device: &Device, surface: &Surface) -> Self {
        let pipeline = build_pipeline(device, |stages, layout| {
            device.create_premultiplied_alpha_pipeline(stages, layout, surface)
        });
        Self::with_pipeline(device, pipeline)
    }

    /// A renderer for passes targeting an offscreen texture of `format`.
    #[must_use]
    pub fn new_offscreen(device: &Device, format: ColorFormat) -> Self {
        let pipeline = build_pipeline(device, |stages, layout| {
            device.create_premultiplied_alpha_pipeline_for_offscreen(stages, layout, format)
        });
        Self::with_pipeline(device, pipeline)
    }

    fn with_pipeline(device: &Device, pipeline: RenderPipeline) -> Self {
        Self {
            pipeline,
            sampler: device.create_clamp_sampler(),
        }
    }

    /// Uploads `batches` for drawing through `view`. Call before opening
    /// the render pass. A batch whose texture is not in `textures` (not
    /// loaded yet, or unregistered) is skipped.
    #[must_use]
    pub fn prepare(
        &self,
        device: &Device,
        view: SpriteView,
        batches: &[SpriteBatch],
        textures: &TextureRegistry,
    ) -> PreparedSprites {
        let camera_buffer = device.create_buffer(16, BufferUsage::Uniform);
        device.write_buffer(&camera_buffer, &camera_bytes(view.camera));

        let mut bytes = Vec::new();
        let mut runs = Vec::new();
        let mut first = 0u32;
        for batch in batches.iter().filter(|b| !b.vertices.is_empty()) {
            let Some(texture) = textures.get(batch.texture) else {
                continue;
            };
            let scissor = match batch.clip {
                Some(clip) => match clip.to_scissor(view.camera, view.viewport) {
                    Some(rect) => Some(rect),
                    None => continue,
                },
                None => None,
            };
            batch
                .vertices
                .iter()
                .for_each(|v| write_vertex(v, &mut bytes));
            let count = u32::try_from(batch.vertices.len()).unwrap_or(u32::MAX);
            let bind_group = device.create_textured_bind_group(
                &self.pipeline,
                &camera_buffer,
                texture,
                &self.sampler,
            );
            runs.push(SpriteRun {
                bind_group,
                vertices: first..first.saturating_add(count),
                scissor,
            });
            first = first.saturating_add(count);
        }
        PreparedSprites {
            vertices: upload_vertices(device, &bytes),
            runs,
            viewport: view.viewport,
        }
    }

    /// Records one draw call per prepared run, in order, into `pass`.
    pub fn draw(&self, pass: &mut RenderPass<'_>, prepared: &PreparedSprites) {
        let Some(vertices) = &prepared.vertices else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, vertices);
        let [width, height] = prepared.viewport;
        for run in &prepared.runs {
            let scissor = run.scissor.unwrap_or(ScissorRect {
                x: 0,
                y: 0,
                width,
                height,
            });
            pass.set_scissor_rect(scissor.x, scissor.y, scissor.width, scissor.height);
            pass.set_bind_group(0, &run.bind_group);
            pass.draw(run.vertices.clone());
        }
        pass.set_scissor_rect(0, 0, width, height);
    }
}

/// Compiles the sprite shader and hands it to `create` with the sprite
/// vertex layout.
fn build_pipeline(
    device: &Device,
    create: impl FnOnce(&ShaderStages<'_>, &VertexLayout) -> RenderPipeline,
) -> RenderPipeline {
    let shader = device.create_shader("meridian-sprite", SPRITE_SHADER_WGSL);
    let stages = ShaderStages {
        shader: &shader,
        vertex_entry: "vs_main",
        fragment_entry: "fs_main",
    };
    create(&stages, &sprite_vertex_layout())
}

/// `[left, right, bottom, top]` as the shader's `vec4`.
fn camera_bytes(camera: Camera2d) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    for (chunk, value) in bytes.chunks_exact_mut(4).zip(camera.extents) {
        chunk.copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn upload_vertices(device: &Device, bytes: &[u8]) -> Option<Buffer> {
    if bytes.is_empty() {
        return None;
    }
    let buffer = device.create_buffer(bytes.len(), BufferUsage::Vertex);
    device.write_buffer(&buffer, bytes);
    Some(buffer)
}
