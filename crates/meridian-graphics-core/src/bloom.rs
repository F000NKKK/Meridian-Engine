//! Bloom: a glow around bright (`Material::emissive`) surfaces — the
//! first piece of docs/graphics-design.md's "Post Processing" scaffold.
//! Built entirely on primitives [`crate::submission`] and
//! `graphics-driver` already expose: an emissive-only redraw of the same
//! [`crate::submission::DrawBuffers`] the main pass baked, a separable
//! Gaussian blur, and an additive composite back onto the main render
//! target.
//!
//! **Pipeline, once per frame a bloom-enabled view renders:**
//! 1. [`SceneRenderer::draw`] — the ordinary lit/unlit scene, straight
//!    into the real render target (swapchain or an offscreen color
//!    texture).
//! 2. [`BloomPass::apply`]'s bright pass — the *same* `DrawBuffers`
//!    redrawn through this module's own emissive-extraction pipelines
//!    (built here, not on `SceneRenderer` — see [`BloomPass::new`]'s doc
//!    comment for why) into an offscreen "bright" texture. A material
//!    with `emissive = [0,0,0]` (the default) contributes solid black
//!    here — harmless once additively composited in step 4 — so no
//!    per-material bloom flag or threshold is needed; `emissive` itself
//!    *is* the mask.
//! 3. Two blur passes (horizontal, then vertical — the standard
//!    separable-Gaussian trick: an `N*N` blur becomes two `N`-tap passes)
//!    ping-ponging between two same-sized offscreen textures, each a
//!    full-screen-triangle draw sampling the previous pass's texture.
//! 4. The composite pass: the blurred bright texture, scaled by
//!    [`BloomConfig::intensity`], additively blended onto the *same*
//!    view the main pass (step 1) already drew into — via
//!    `CommandBuffer::begin_render_pass_loaded`, which preserves existing
//!    contents instead of clearing them.
//!
//! **Deliberately not HDR.** Every offscreen texture here (bright/ping/
//! pong) is [`ColorFormat::SrgbRgba8`], the same format `TextureRegistry`
//! uses for uploaded textures — an emissive value is whatever a
//! `Material` sets (no bloom-specific tone mapping or exposure control),
//! clamped to `[0,1]` like every other color in this bridge. A true HDR
//! pipeline (`Rgba16Float` intermediate targets, exposure/tone-mapping)
//! is real future work if a scene ever needs emissive values above `1.0`
//! to look right — it would only mean adding a variant to
//! `graphics-driver::ColorFormat` and passing it through, not a redesign
//! here; the visible glow this produces today does not depend on it.
//!
//! **Fixed at construction size**, matching the swapchain/target it was
//! built for — resize by constructing a new [`BloomPass`] (cheap: a
//! handful of small offscreen textures and three tiny pipelines), the
//! same "rebuild on resize" pattern `DepthTexture` already uses in every
//! windowed example.

use meridian_graphics_driver::{
    BindGroup, BufferUsage, ColorFormat, DepthTexture, Device, RenderPipeline, Sampler, Surface,
    SurfaceFrame, Texture,
};

use crate::SceneRenderer;
use crate::submission::{
    DrawBuffers, DrawKind, EMISSIVE_EXTRACT_SHADER_WGSL, emissive_from_colored_layout,
    emissive_from_textured_layout,
};

/// Every offscreen texture [`BloomPass`] creates uses this format — one
/// value threaded through both texture and pipeline creation (see
/// `graphics-driver::ColorFormat`'s own doc comment for why that
/// matters: a mismatch between the two is exactly the `wgpu` validation
/// error this module used to be able to trigger before the format
/// became one shared value instead of two independently-hardcoded
/// assumptions).
const OFFSCREEN_FORMAT: ColorFormat = ColorFormat::SrgbRgba8;

/// A full-screen-triangle vertex shader shared by both blur directions
/// and the composite pass — see `graphics-driver`'s
/// `Device::create_fullscreen_pipeline` doc comment for why no vertex
/// buffer is needed.
const FULLSCREEN_VERTEX_WGSL: &str = r#"
struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let p = positions[index];
    var out: VsOut;
    out.clip_position = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, 1.0 - (p.y + 1.0) * 0.5);
    return out;
}
"#;

/// A 5-tap separable Gaussian blur (the classic LearnOpenGL weights),
/// applied along `u.direction` — call once with a horizontal direction,
/// once with vertical, ping-ponging the source/destination textures, for
/// a full 2D blur at roughly half the cost of a naive 2D kernel.
fn blur_shader_wgsl() -> String {
    format!(
        r#"
{FULLSCREEN_VERTEX_WGSL}

struct BlurUniforms {{
    texel_size: vec2<f32>,
    direction: vec2<f32>,
}};

@group(0) @binding(0)
var<uniform> u: BlurUniforms;
@group(0) @binding(1)
var src_tex: texture_2d<f32>;
@group(0) @binding(2)
var src_sampler: sampler;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {{
    let weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    // A 5-tap kernel at a 1-texel stride only spreads a few pixels wide
    // at full render resolution — imperceptible next to a whole shape.
    // `SPREAD` stretches the same 5 taps across a much wider footprint
    // (each pass is still only 2 ping-pong textures; two directions of
    // one wide separable pass, same cost as before, just farther apart).
    let spread = 6.0;
    var result = textureSample(src_tex, src_sampler, in.uv).rgb * weights[0];
    for (var i = 1; i < 5; i = i + 1) {{
        let offset = u.direction * u.texel_size * f32(i) * spread;
        result += textureSample(src_tex, src_sampler, in.uv + offset).rgb * weights[i];
        result += textureSample(src_tex, src_sampler, in.uv - offset).rgb * weights[i];
    }}
    return vec4<f32>(result, 1.0);
}}
"#
    )
}

/// Samples the blurred bright texture, scaled by `u.intensity`, additive
/// onto whatever [`meridian_graphics_driver::CommandBuffer::begin_render_pass_loaded`]
/// targets.
fn composite_shader_wgsl() -> String {
    format!(
        r#"
{FULLSCREEN_VERTEX_WGSL}

struct CompositeUniforms {{
    intensity: vec4<f32>,
}};

@group(0) @binding(0)
var<uniform> u: CompositeUniforms;
@group(0) @binding(1)
var bloom_tex: texture_2d<f32>;
@group(0) @binding(2)
var bloom_sampler: sampler;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {{
    let sampled = textureSample(bloom_tex, bloom_sampler, in.uv);
    return vec4<f32>(sampled.rgb * u.intensity.x, 1.0);
}}
"#
    )
}

/// The `BlurUniforms` bytes the blur shader reads: `texel_size` (1/width,
/// 1/height) then `direction` (`(1,0)` horizontal, `(0,1)` vertical).
fn blur_uniform_bytes(width: u32, height: u32, horizontal: bool) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0..4].copy_from_slice(&(1.0 / width as f32).to_le_bytes());
    bytes[4..8].copy_from_slice(&(1.0 / height as f32).to_le_bytes());
    bytes[8..12].copy_from_slice(&(if horizontal { 1.0f32 } else { 0.0f32 }).to_le_bytes());
    bytes[12..16].copy_from_slice(&(if horizontal { 0.0f32 } else { 1.0f32 }).to_le_bytes());
    bytes
}

/// Tunable bloom parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BloomConfig {
    /// Multiplies the blurred bright texture before the additive
    /// composite — `0.0` disables the visible effect without skipping
    /// the passes (see [`BloomPass::apply`] for actually skipping them).
    pub intensity: f32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self { intensity: 1.8 }
    }
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
/// them on `SceneRenderer` against the surface's format instead (an
/// earlier version of this code did) is exactly the class of bug
/// `graphics-driver::ColorFormat` exists to make structurally
/// impossible: a mesh pipeline built for the wrong render pass's format.
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
        let emissive_from_colored_pipeline = device.create_render_pipeline_for_offscreen(
            &emissive_shader,
            "vs_main",
            "fs_main",
            &emissive_from_colored_layout(),
            OFFSCREEN_FORMAT,
            true,
        );
        let emissive_from_textured_pipeline = device.create_render_pipeline_for_offscreen(
            &emissive_shader,
            "vs_main",
            "fs_main",
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

        let sampler = device.create_sampler();

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
        pass: &mut meridian_graphics_driver::RenderPass<'_>,
        buffers: &[DrawBuffers],
    ) {
        // Two bind groups, not one: with `layout: None` (auto-derived
        // bind group layout), wgpu treats each pipeline's layout as a
        // distinct object even when two shaders declare an identical
        // `@group`/`@binding` shape — a bind group built against one
        // pipeline is rejected ("exclusive pipelines don't match") the
        // moment a *different* pipeline is bound, which is exactly what
        // reusing one bind group across the colored/textured pipeline
        // switch below used to do. Both rebuilt every frame, same
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

    fn blur_uniform_bytes(&self, horizontal: bool) -> [u8; 16] {
        blur_uniform_bytes(self.width, self.height, horizontal)
    }

    /// Runs the full bloom pipeline for one view and composites the
    /// result onto `target_frame` — the *same* frame `renderer`'s main
    /// [`SceneRenderer::draw`] already rendered into earlier in this
    /// `command_buffer`, per the module doc's step 4.
    ///
    /// `config.intensity <= 0.0` skips every pass (nothing to add), so a
    /// caller can toggle bloom off per-frame without reconstructing this
    /// `BloomPass`.
    pub fn apply(
        &self,
        device: &Device,
        command_buffer: &mut meridian_graphics_driver::CommandBuffer<'_>,
        renderer: &SceneRenderer,
        draw_buffers: &[DrawBuffers],
        target_frame: &SurfaceFrame,
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
            self.draw_emissive(device, renderer, &mut pass, draw_buffers);
        }

        // Step 3: horizontal blur (bright -> ping), then vertical (ping -> pong).
        let horizontal_uniform = device.create_buffer(16, BufferUsage::Uniform);
        device.write_buffer(&horizontal_uniform, &self.blur_uniform_bytes(true));
        let horizontal_bind_group = device.create_textured_bind_group(
            &self.blur_pipeline,
            &horizontal_uniform,
            &self.bright,
            &self.sampler,
        );
        {
            let mut pass =
                command_buffer.begin_render_pass(self.ping.view(), [0.0, 0.0, 0.0, 1.0], None);
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &horizontal_bind_group);
            pass.draw(0..3);
        }

        let vertical_uniform = device.create_buffer(16, BufferUsage::Uniform);
        device.write_buffer(&vertical_uniform, &self.blur_uniform_bytes(false));
        let vertical_bind_group = device.create_textured_bind_group(
            &self.blur_pipeline,
            &vertical_uniform,
            &self.ping,
            &self.sampler,
        );
        {
            let mut pass =
                command_buffer.begin_render_pass(self.pong.view(), [0.0, 0.0, 0.0, 1.0], None);
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &vertical_bind_group);
            pass.draw(0..3);
        }

        // A second horizontal+vertical pass (pong -> ping -> pong), same
        // wide-spread kernel — one pass alone reads as a thin fuzzy edge;
        // stacking two widens the falloff into an actual halo, at the
        // cost of two more small fullscreen-triangle draws.
        let horizontal_uniform_2 = device.create_buffer(16, BufferUsage::Uniform);
        device.write_buffer(&horizontal_uniform_2, &self.blur_uniform_bytes(true));
        let horizontal_bind_group_2 = device.create_textured_bind_group(
            &self.blur_pipeline,
            &horizontal_uniform_2,
            &self.pong,
            &self.sampler,
        );
        {
            let mut pass =
                command_buffer.begin_render_pass(self.ping.view(), [0.0, 0.0, 0.0, 1.0], None);
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &horizontal_bind_group_2);
            pass.draw(0..3);
        }

        let vertical_uniform_2 = device.create_buffer(16, BufferUsage::Uniform);
        device.write_buffer(&vertical_uniform_2, &self.blur_uniform_bytes(false));
        let vertical_bind_group_2 = device.create_textured_bind_group(
            &self.blur_pipeline,
            &vertical_uniform_2,
            &self.ping,
            &self.sampler,
        );
        {
            let mut pass =
                command_buffer.begin_render_pass(self.pong.view(), [0.0, 0.0, 0.0, 1.0], None);
            pass.set_pipeline(&self.blur_pipeline);
            pass.set_bind_group(0, &vertical_bind_group_2);
            pass.draw(0..3);
        }

        // Step 4: additive composite onto the already-drawn target.
        let mut intensity_bytes = [0u8; 16];
        intensity_bytes[0..4].copy_from_slice(&self.config.intensity.to_le_bytes());
        let composite_uniform = device.create_buffer(16, BufferUsage::Uniform);
        device.write_buffer(&composite_uniform, &intensity_bytes);
        let composite_bind_group: BindGroup = device.create_textured_bind_group(
            &self.composite_pipeline,
            &composite_uniform,
            &self.pong,
            &self.sampler,
        );
        {
            let mut pass = command_buffer.begin_render_pass_loaded(target_frame.view());
            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &composite_bind_group);
            pass.draw(0..3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `BloomPass::new`/`apply` need a real windowed `Surface`/GPU device
    /// (the same convention as `SceneRenderer` — see `submission.rs`'s
    /// own tests module doc comment); this test covers the one piece
    /// that's pure CPU math: the blur uniform's byte layout, which the
    /// WGSL side reads as `vec2<f32> texel_size, vec2<f32> direction`.
    #[test]
    fn blur_uniform_bytes_encode_texel_size_and_direction() {
        let horizontal = blur_uniform_bytes(100, 200, true);
        assert_eq!(
            f32::from_le_bytes(horizontal[0..4].try_into().unwrap()),
            0.01
        );
        assert_eq!(
            f32::from_le_bytes(horizontal[4..8].try_into().unwrap()),
            0.005
        );
        assert_eq!(
            f32::from_le_bytes(horizontal[8..12].try_into().unwrap()),
            1.0
        );
        assert_eq!(
            f32::from_le_bytes(horizontal[12..16].try_into().unwrap()),
            0.0
        );

        let vertical = blur_uniform_bytes(100, 200, false);
        assert_eq!(f32::from_le_bytes(vertical[8..12].try_into().unwrap()), 0.0);
        assert_eq!(
            f32::from_le_bytes(vertical[12..16].try_into().unwrap()),
            1.0
        );
    }
}
