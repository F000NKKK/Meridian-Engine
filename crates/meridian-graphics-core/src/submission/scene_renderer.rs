//! Pipelines and state that render a 3-D scene.

use super::DrawBuffers;
use super::DrawKind;
use super::DrawRequest;
use super::MAX_LIGHTS;
use super::PreparedScene;
use super::SceneAssets;
use super::ShadowDepthPass;
use super::ShadowLight;
use super::TEXTURED_SHADER_WGSL;
use super::UNLIT_SHADER_WGSL;
use super::ViewUniforms;
use super::bake_draw_buffers;
use super::colored_vertex_layout;
use super::directional_shadow_camera;
use super::lit_shader_wgsl;
use super::lit_textured_shader_wgsl;
use super::lit_uniform_bytes;
use super::mat4_to_bytes;
use super::textured_vertex_layout;
use crate::scene::{Light, Renderable3d, Scene3d};
use meridian_gac_core::Vec3;
use meridian_graphics_driver::{
    BindGroup, Buffer, BufferUsage, CommandBuffer, Device, RenderPass as DriverRenderPass,
    RenderPipeline, Sampler, ShaderStages, Surface, Texture, VertexLayout,
};

/// Owns all four main pipelines (colored/textured × lit/unlit) plus the
/// the shared view/lighting uniform, and the sampler for rendering a
/// [`Scene3d`] view. One instance per view (a UI overlay's `Scene2d` is
/// a separate, future-work sibling — see docs/graphics-design.md);
/// building it needs a real GPU device and a surface to match color/
/// depth formats against. The emissive-extraction pipelines
/// [`crate::bloom::BloomPass`] draws through are *not* owned here — they
/// render into `BloomPass`'s own offscreen texture, whose format is
/// `BloomPass`'s to pick, not something this type should guess at (see
/// `bloom.rs`'s module doc and `uniform_buffer`(Self::uniform_buffer)).
#[derive(Debug)]
pub struct SceneRenderer {
    colored_unlit_pipeline: RenderPipeline,
    colored_unlit_bind_group: BindGroup,
    colored_lit_pipeline: RenderPipeline,
    colored_lit_bind_group: BindGroup,
    pub(crate) textured_unlit_pipeline: RenderPipeline,
    pub(crate) textured_lit_pipeline: RenderPipeline,
    pub(crate) uniform_buffer: Buffer,
    pub(crate) sampler: Sampler,
    /// A `Depth32Float` texture, both a render target (the shadow depth
    /// pass writes into it) and sampleable (the lit shaders' shadow
    /// lookup reads it) — see
    /// `graphics-driver::Device::create_shadow_map_texture`'s own doc
    /// comment for why [`Texture`] rather than the plain
    /// render-target-only `DepthTexture`.
    pub(crate) shadow_map: Texture,
    pub(crate) shadow_sampler: Sampler,
    shadow_depth: ShadowDepthPass,
}

/// [`SceneRenderer::new`]'s default shadow map resolution — generous
/// enough that both example scenes' shadows don't read as visibly
/// blocky, without the per-frame cost a much larger map would add. A
/// caller that wants sharper (or cheaper, blockier) shadows picks their
/// own value via [`SceneRenderer::with_shadow_map_size`] instead.
pub const DEFAULT_SHADOW_MAP_SIZE: u32 = 2048;

impl SceneRenderer {
    /// Builds a renderer with [`DEFAULT_SHADOW_MAP_SIZE`] — see
    /// [`with_shadow_map_size`](Self::with_shadow_map_size) to pick a
    /// specific resolution (a larger map reduces blocky/aliased shadow
    /// edges at a real per-frame GPU cost; a smaller one is cheaper and
    /// blockier — there's no single right answer across every scene's
    /// scale and hardware budget, so this is an explicit, disclosed
    /// application choice, not a hardcoded constant with no escape
    /// hatch).
    pub fn new(device: &Device, surface: &Surface) -> Self {
        Self::with_shadow_map_size(device, surface, DEFAULT_SHADOW_MAP_SIZE)
    }

    pub fn with_shadow_map_size(device: &Device, surface: &Surface, shadow_map_size: u32) -> Self {
        let build = |label: &str, wgsl: &str, layout: &VertexLayout| {
            main_pipeline(device, surface, label, wgsl, layout)
        };
        let colored_layout = colored_vertex_layout();
        let textured_layout = textured_vertex_layout();
        let colored_unlit_pipeline =
            build("meridian-unlit-colored", UNLIT_SHADER_WGSL, &colored_layout);
        let colored_lit_pipeline =
            build("meridian-lit-colored", &lit_shader_wgsl(), &colored_layout);
        let textured_unlit_pipeline = build(
            "meridian-unlit-textured",
            TEXTURED_SHADER_WGSL,
            &textured_layout,
        );
        let textured_lit_pipeline = build(
            "meridian-lit-textured",
            &lit_textured_shader_wgsl(),
            &textured_layout,
        );

        let shadow_map = device.create_shadow_map_texture(shadow_map_size);
        let shadow_sampler = device.create_comparison_sampler();
        let shadow_depth = ShadowDepthPass::new(device);

        // 416 bytes: see `lit_uniform_bytes`'s doc comment for the
        // layout. The unlit pipelines only read the first 64 (view_proj)
        // but share this same buffer — one write serves every pipeline.
        let uniform_buffer = device.create_buffer(416, BufferUsage::Uniform);
        let colored_unlit_bind_group =
            device.create_uniform_bind_group(&colored_unlit_pipeline, &uniform_buffer);
        let colored_lit_bind_group = device.create_shadow_bind_group(
            &colored_lit_pipeline,
            &uniform_buffer,
            &shadow_map,
            &shadow_sampler,
        );
        let sampler = device.create_sampler();

        let renderer = Self {
            colored_unlit_pipeline,
            colored_unlit_bind_group,
            colored_lit_pipeline,
            colored_lit_bind_group,
            textured_unlit_pipeline,
            textured_lit_pipeline,
            uniform_buffer,
            sampler,
            shadow_map,
            shadow_sampler,
            shadow_depth,
        };

        renderer.clear_shadow_map(device);
        renderer
    }

    /// One throwaway pass, no draws, purely for its `LoadOp::Clear(1.0)`
    /// side effect — guarantees the shadow map always reads back as
    /// "no occluder anywhere" (far plane depth) even for a caller
    /// that never calls `render_shadow_pass` (e.g. a scene with no
    /// directional light), rather than leaving it as newly-allocated
    /// GPU memory with undefined content.
    fn clear_shadow_map(&self, device: &Device) {
        let mut init_commands = device.create_command_buffer();
        {
            let _clear_only = init_commands.begin_shadow_pass(&self.shadow_map);
        }
        init_commands.submit();
    }

    /// The shared view/lighting uniform buffer — [`crate::bloom::BloomPass`]
    /// binds its own emissive-extraction pipelines against this same
    /// buffer (it needs `view_proj` for its vertex shader), rebuilding a
    /// fresh bind group against it each frame, the same "correct now,
    /// cache later" trade-off already applied to per-texture bind groups
    /// (see the module doc).
    pub(crate) fn uniform_buffer(&self) -> &Buffer {
        &self.uniform_buffer
    }

    /// Writes this view's camera, the shadow-casting light's own
    /// view-projection matrix, ambient and lights into the shared
    /// uniform — call once per frame before [`draw`](Self::draw), after
    /// [`render_shadow_pass`](Self::render_shadow_pass) (which computes
    /// the [`ShadowLight`]). Lights beyond
    /// [`MAX_LIGHTS`](super::MAX_LIGHTS) are dropped and logged.
    pub fn set_view(&self, device: &Device, view: &ViewUniforms<'_>) {
        if view.lights.len() > MAX_LIGHTS {
            meridian_foundation::log_warn!(
                "scene has {} lights, dropping {} beyond the cap of {MAX_LIGHTS}",
                view.lights.len(),
                view.lights.len() - MAX_LIGHTS
            );
        }
        device.write_buffer(&self.uniform_buffer, &lit_uniform_bytes(view));
    }

    /// Builds one [`DrawBuffers`](super::DrawBuffers) per renderable whose mesh/material both
    /// resolve (a renderable naming a missing handle is silently skipped
    /// — the caller controls registry contents, so a miss here means an
    /// asset hasn't loaded yet, not a bug to panic over). Call this
    /// *before* opening the render pass; pass the result to
    /// [`draw`](Self::draw) once inside it.
    pub fn prepare(
        &self,
        device: &Device,
        renderables: &[Renderable3d],
        assets: &SceneAssets<'_>,
    ) -> Vec<DrawBuffers> {
        renderables
            .iter()
            .filter_map(|renderable| {
                let source = assets.meshes.get(renderable.mesh)?;
                let material = assets.materials.get(renderable.material)?;
                let albedo = material
                    .albedo
                    .and_then(|handle| assets.textures.get(handle));
                Some(bake_draw_buffers(
                    device,
                    self,
                    &DrawRequest {
                        mesh: source,
                        material,
                        frame: &renderable.frame,
                        albedo,
                        casts_shadow: renderable.casts_shadow,
                    },
                ))
            })
            .collect()
    }

    /// The shadow-casting depth pass: finds `scene`'s first
    /// [`Light::Directional`] (if any), builds its
    /// [`directional_shadow_camera`](super::directional_shadow_camera), writes that light-space matrix
    /// into the (separate) shadow uniform buffer, bakes `scene`'s draw
    /// buffers (reused afterward by [`draw`](Self::draw) — never
    /// rebuilt), and records a depth-only pass into the internal shadow
    /// map through `commands`. Returns the baked buffers plus the
    /// [`ShadowLight`] for the caller to pass
    /// into [`set_view`](Self::set_view) before its own (separate, later)
    /// main color pass — see this module's own doc for why the shadow
    /// pass can't be folded into [`submit_scene3d`](super::submit_scene3d) itself (it has to
    /// finish, as its own pass, before the caller's main pass opens on
    /// the same `commands`).
    ///
    /// No [`Light::Directional`] in `scene.lights`: still records an
    /// (empty-of-occluders-mattering) depth pass for consistency, and
    /// returns `shadow_caster = u32::MAX`, which `shade`'s lookup never
    /// matches — every light renders fully lit, the same as before
    /// shadows existed.
    pub fn render_shadow_pass(
        &self,
        device: &Device,
        commands: &mut CommandBuffer<'_>,
        scene: &Scene3d,
        assets: &SceneAssets<'_>,
    ) -> PreparedScene {
        let shadow = shadow_light_for(scene);
        device.write_buffer(
            &self.shadow_depth.uniform_buffer,
            &mat4_to_bytes(shadow.view_proj),
        );
        let buffers = self.prepare(device, &scene.renderables, assets);
        self.record_shadow_casters(commands, &buffers);
        PreparedScene { buffers, shadow }
    }

    /// Records the depth-only pass of every shadow-casting entry.
    fn record_shadow_casters(&self, commands: &mut CommandBuffer<'_>, buffers: &[DrawBuffers]) {
        let mut pass = commands.begin_shadow_pass(&self.shadow_map);
        for entry in buffers.iter().filter(|entry| entry.casts_shadow) {
            let (pipeline, bind_group) = match &entry.kind {
                DrawKind::Colored => (
                    &self.shadow_depth.colored_pipeline,
                    &self.shadow_depth.colored_bind_group,
                ),
                DrawKind::Textured { .. } => (
                    &self.shadow_depth.textured_pipeline,
                    &self.shadow_depth.textured_bind_group,
                ),
            };
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group);
            pass.set_vertex_buffer(0, &entry.vertex_buffer);
            pass.set_index_buffer_u16(&entry.index_buffer);
            pass.draw_indexed(0..entry.index_count);
        }
    }

    /// Records one draw call per `buffers` entry against `pass`,
    /// switching pipeline/bind group per entry's baked family and lit
    /// flag — call inside the render pass, after
    /// [`render_shadow_pass`](Self::render_shadow_pass) built `buffers`
    /// and [`set_view`](Self::set_view) wrote this frame's uniform.
    pub fn draw(&self, pass: &mut DriverRenderPass<'_>, buffers: &[DrawBuffers]) {
        for entry in buffers {
            match &entry.kind {
                DrawKind::Colored => {
                    let pipeline = if entry.lit {
                        &self.colored_lit_pipeline
                    } else {
                        &self.colored_unlit_pipeline
                    };
                    let bind_group = if entry.lit {
                        &self.colored_lit_bind_group
                    } else {
                        &self.colored_unlit_bind_group
                    };
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, bind_group);
                }
                DrawKind::Textured { bind_group } => {
                    let pipeline = if entry.lit {
                        &self.textured_lit_pipeline
                    } else {
                        &self.textured_unlit_pipeline
                    };
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, bind_group);
                }
            }
            pass.set_vertex_buffer(0, &entry.vertex_buffer);
            pass.set_index_buffer_u16(&entry.index_buffer);
            pass.draw_indexed(0..entry.index_count);
        }
    }
}

/// Builds one of the four main pipelines from WGSL source.
fn main_pipeline(
    device: &Device,
    surface: &Surface,
    label: &str,
    wgsl_source: &str,
    layout: &VertexLayout,
) -> RenderPipeline {
    let shader = device.create_shader(label, wgsl_source);
    let stages = ShaderStages {
        shader: &shader,
        vertex_entry: "vs_main",
        fragment_entry: "fs_main",
    };
    device.create_render_pipeline(&stages, layout, surface, true)
}

/// The scene's first directional light as a shadow caster, or
/// `caster = u32::MAX` (matched by no light) when there is none.
fn shadow_light_for(scene: &Scene3d) -> ShadowLight {
    let caster = scene
        .lights
        .iter()
        .position(|l| matches!(l, Light::Directional { .. }))
        .map_or(u32::MAX, |i| i as u32);
    let direction = match scene.lights.get(caster as usize) {
        Some(Light::Directional { direction, .. }) => direction.transform_vector(Vec3::X),
        _ => Vec3::new(0.0, -1.0, 0.0),
    };
    let camera = directional_shadow_camera(direction, Vec3::ZERO);
    ShadowLight {
        view_proj: camera.view_projection_matrix(),
        direction,
        caster,
    }
}
