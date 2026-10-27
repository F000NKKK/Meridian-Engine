//! Extract, blur and composite: the bloom post-process.

use meridian_rendering_domain::BloomConfig;

use super::{BloomInput, BlurDirection, blur_shader_wgsl, composite_shader_wgsl};
use crate::submission::{
    DrawBuffers, DrawKind, EMISSIVE_EXTRACT_SHADER_WGSL, emissive_from_colored_layout,
    emissive_from_textured_layout,
};
use crate::{
    BindGroup, BufferUsage, ColorFormat, DepthTexture, Device, RenderPipeline, Sampler,
    SceneRenderer, ShaderStages, Surface, SurfaceFrame, Texture,
};

/// Every offscreen texture [`BloomPass`] creates uses this format — one
/// value threaded through both texture and pipeline creation (see
/// `rendering-wgpu::ColorFormat`'s own doc comment for why that
/// matters: a mismatch between the two is a `wgpu` validation error,
/// which one shared value makes impossible).
const OFFSCREEN_FORMAT: ColorFormat = ColorFormat::SrgbRgba8;

/// The `BlurUniforms` bytes the blur shader reads: `texel_size` (1/width,
/// 1/height) then `direction` (`(1,0)` horizontal, `(0,1)` vertical).
pub(crate) fn blur_uniform_bytes(width: u32, height: u32, direction: BlurDirection) -> [u8; 16] {
    let [dx, dy] = direction.components();
    let mut bytes = [0u8; 16];
    bytes[0..4].copy_from_slice(&(1.0 / width as f32).to_le_bytes());
    bytes[4..8].copy_from_slice(&(1.0 / height as f32).to_le_bytes());
    bytes[8..12].copy_from_slice(&dx.to_le_bytes());
    bytes[12..16].copy_from_slice(&dy.to_le_bytes());
    bytes
}

/// One blur pass: read `source`, write `target`, smearing along `direction`.
struct BlurStep<'a> {
    source: &'a Texture,
    target: &'a Texture,
    direction: BlurDirection,
}

/// Owns bloom's offscreen textures and its three small pipelines
/// (emissive extraction x2 — colored/textured, blur, composite), sized
/// for one `width`x`height` render target — see the module doc for the
/// four-step pipeline and why this isn't HDR.
///
/// The emissive-extraction pipelines live here, not on `SceneRenderer`,
/// deliberately: they only ever render into *this* struct's own
/// `bright` texture, never the swapchain, so the format they're built
/// against has to be `BloomPass`'s own `OFFSCREEN_FORMAT` — building
/// them on `SceneRenderer` against the surface's format instead is the
/// class of bug
/// `rendering-wgpu::ColorFormat` exists to make structurally
/// impossible: a mesh pipeline built for the wrong render pass's format.
#[derive(Debug)]
pub struct BloomPass {
    bright: Texture,
    /// Depth buffer for the bright pass only — cleared and depth-tested
    /// fresh each frame against just the emissive redraw (step 2), so a
    /// shape fully hidden behind another opaque shape (from the camera's
    /// point of view) does not still contribute its glow color to
    /// `bright`. Without this, the bright pass drew every emissive
    /// renderable unoccluded, and the additive composite (step 4) then
    /// bled a "ghost" of a shape's color through whatever opaquely
    /// occluded it in the real scene — the concrete bug this was written
    /// to fix (orbiting shapes appearing to glow *through* each other).
    /// Not shared with the main scene's own depth texture: this pass
    /// draws the exact same triangles with the exact same `view_proj`,
    /// so a self-contained depth test among just those draws already
    /// reconstructs the correct occlusion, with no cross-texture size/
    /// format coupling needed.
    bright_depth: DepthTexture,
    ping: Texture,
    pong: Texture,
    emissive_from_colored_pipeline: RenderPipeline,
    emissive_from_textured_pipeline: RenderPipeline,
    blur_pipeline: RenderPipeline,
    composite_pipeline: RenderPipeline,
    sampler: Sampler,
    width: u32,
    height: u32,
    pub config: BloomConfig,
}

impl BloomPass {
    /// `surface` supplies the composite pass's target color format (the
    /// real swapchain format) — `width`/`height` should match it too,
    /// so the bloom textures are the same resolution as what's actually
    /// being rendered.
    pub fn new(device: &Device, width: u32, height: u32, surface: &Surface) -> Self {
        let bright = device.create_offscreen_color_texture(width, height, OFFSCREEN_FORMAT);
        let bright_depth = device.create_depth_texture(width, height);
        let ping = device.create_offscreen_color_texture(width, height, OFFSCREEN_FORMAT);
        let pong = device.create_offscreen_color_texture(width, height, OFFSCREEN_FORMAT);

        let emissive_shader =
            device.create_shader("meridian-emissive-extract", EMISSIVE_EXTRACT_SHADER_WGSL);
        let emissive_stages = ShaderStages {
            shader: &emissive_shader,
            vertex_entry: "vs_main",
            fragment_entry: "fs_main",
        };
        let emissive_from_colored_pipeline = device.create_render_pipeline_for_offscreen(
            &emissive_stages,
            &emissive_from_colored_layout(),
            OFFSCREEN_FORMAT,
            true,
        );
        let emissive_from_textured_pipeline = device.create_render_pipeline_for_offscreen(
            &emissive_stages,
            &emissive_from_textured_layout(),
            OFFSCREEN_FORMAT,
            true,
        );

        let blur_shader = device.create_shader("meridian-bloom-blur", &blur_shader_wgsl());
        // The bright/ping/pong textures all share OFFSCREEN_FORMAT; the
        // blur pass's own target format matches them, independent of
        // the composite pass's target format below (the real swapchain
        // format).
        let blur_pipeline = device.create_fullscreen_pipeline_for_offscreen(
            &blur_shader,
            "fs_main",
            OFFSCREEN_FORMAT,
            false,
        );

        let composite_shader =
            device.create_shader("meridian-bloom-composite", &composite_shader_wgsl());
        let composite_pipeline =
            device.create_fullscreen_pipeline(&composite_shader, "fs_main", surface, true);

        // Clamp, not repeat: the blur kernel samples beyond `[0,1]` near
        // the screen edges (see `Device::create_clamp_sampler`'s doc
        // comment) — `Repeat` there wrapped a shape's bloom halo around
        // to the opposite edge of the screen.
        let sampler = device.create_clamp_sampler();

        Self {
            bright,
            bright_depth,
            ping,
            pong,
            emissive_from_colored_pipeline,
            emissive_from_textured_pipeline,
            blur_pipeline,
            composite_pipeline,
            sampler,
            width,
            height,
            config: BloomConfig::default(),
        }
    }

    /// Redraws `buffers` through this `BloomPass`'s own emissive-
    /// extraction pipelines into `self.bright` — step 2 of the module
    /// doc's pipeline. `renderer` supplies only the shared view/lighting
    /// uniform buffer (for `view_proj`); the pipelines themselves are
    /// this struct's, not `renderer`'s (see [`BloomPass::new`]'s doc
    /// comment for why).
    fn draw_emissive(
        &self,
        device: &Device,
        renderer: &SceneRenderer,
        pass: &mut crate::RenderPass<'_>,
        buffers: &[DrawBuffers],
    ) {
        // Two bind groups, not one: with `layout: None` (auto-derived
        // bind group layout), wgpu treats each pipeline's layout as a
        // distinct object even when two shaders declare an identical
        // `@group`/`@binding` shape — a bind group built against one
        // pipeline is rejected ("exclusive pipelines don't match") the
        // moment a *different* pipeline is bound, as the colored/textured
        // pipeline switch below does. Both rebuilt every frame, same
        // "correct now, cache later" trade-off as the per-texture bind
        // groups in `submission.rs`.
        let colored_bind_group = device.create_uniform_bind_group(
            &self.emissive_from_colored_pipeline,
            renderer.uniform_buffer(),
        );
        let textured_bind_group = device.create_uniform_bind_group(
            &self.emissive_from_textured_pipeline,
            renderer.uniform_buffer(),
        );
        for entry in buffers {
            let (pipeline, bind_group) = match &entry.kind {
                DrawKind::Colored => (&self.emissive_from_colored_pipeline, &colored_bind_group),
                DrawKind::Textured { .. } => {
                    (&self.emissive_from_textured_pipeline, &textured_bind_group)
                }
            };
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group);
            pass.set_vertex_buffer(0, &entry.vertex_buffer);
            pass.set_index_buffer_u16(&entry.index_buffer);
            pass.draw_indexed(0..entry.index_count);
        }
    }

    /// Runs the full bloom pipeline for one view and composites the
    /// result onto `input.target_frame` — the *same* frame `input.renderer`'s
    /// main [`SceneRenderer::draw`] already rendered into earlier in this
    /// `command_buffer`, per the module doc's step 4.
    ///
    /// `config.intensity <= 0.0` skips every pass (nothing to add), so a
    /// caller can toggle bloom off per-frame without reconstructing this
    /// `BloomPass`.
    pub fn apply(
        &self,
        device: &Device,
        command_buffer: &mut crate::CommandBuffer<'_>,
        input: &BloomInput<'_>,
    ) {
        if self.config.intensity <= 0.0 {
            return;
        }

        // Step 2: emissive-only bright pass.
        {
            let mut pass = command_buffer.begin_render_pass(
                self.bright.view(),
                [0.0, 0.0, 0.0, 1.0],
                Some(&self.bright_depth),
            );
            self.draw_emissive(device, input.renderer, &mut pass, input.draw_buffers);
        }

        // Step 3: horizontal blur (bright -> ping), then vertical (ping -> pong).
        self.blur(
            device,
            command_buffer,
            &BlurStep {
                source: &self.bright,
                target: &self.ping,
                direction: BlurDirection::Horizontal,
            },
        );
        self.blur(
            device,
            command_buffer,
            &BlurStep {
                source: &self.ping,
                target: &self.pong,
                direction: BlurDirection::Vertical,
            },
        );
        // A second horizontal+vertical pass (pong -> ping -> pong), same
        // wide-spread kernel — one pass alone reads as a thin fuzzy edge;
        // stacking two widens the falloff into an actual halo, at the
        // cost of two more small fullscreen-triangle draws.
        self.blur(
            device,
            command_buffer,
            &BlurStep {
                source: &self.pong,
                target: &self.ping,
                direction: BlurDirection::Horizontal,
            },
        );
        self.blur(
            device,
            command_buffer,
            &BlurStep {
                source: &self.ping,
                target: &self.pong,
                direction: BlurDirection::Vertical,
            },
        );

        // Step 4: additive composite onto the already-drawn target.
        self.composite(device, command_buffer, input.target_frame);
    }

    /// Records one fullscreen blur draw.
    fn blur(
        &self,
        device: &Device,
        command_buffer: &mut crate::CommandBuffer<'_>,
        step: &BlurStep<'_>,
    ) {
        let uniform = device.create_buffer(16, BufferUsage::Uniform);
        device.write_buffer(
            &uniform,
            &blur_uniform_bytes(self.width, self.height, step.direction),
        );
        let bind_group = device.create_textured_bind_group(
            &self.blur_pipeline,
            &uniform,
            step.source,
            &self.sampler,
        );
        let mut pass =
            command_buffer.begin_render_pass(step.target.view(), [0.0, 0.0, 0.0, 1.0], None);
        pass.set_pipeline(&self.blur_pipeline);
        pass.set_bind_group(0, &bind_group);
        pass.draw(0..3);
    }

    /// Adds the blurred bright pass onto `target_frame`.
    fn composite(
        &self,
        device: &Device,
        command_buffer: &mut crate::CommandBuffer<'_>,
        target_frame: &SurfaceFrame,
    ) {
        let mut intensity_bytes = [0u8; 16];
        intensity_bytes[0..4].copy_from_slice(&self.config.intensity.to_le_bytes());
        let uniform = device.create_buffer(16, BufferUsage::Uniform);
        device.write_buffer(&uniform, &intensity_bytes);
        let bind_group: BindGroup = device.create_textured_bind_group(
            &self.composite_pipeline,
            &uniform,
            &self.pong,
            &self.sampler,
        );
        let mut pass = command_buffer.begin_render_pass_loaded(target_frame.view());
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &bind_group);
        pass.draw(0..3);
    }
}
