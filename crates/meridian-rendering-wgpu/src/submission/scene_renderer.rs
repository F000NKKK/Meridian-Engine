//! Pipelines and state that render a 3-D scene.

use meridian_rendering_domain::scene::Scene3d;
use meridian_rendering_domain::{EnvironmentMap, ShadowLight, ViewUniforms};

use super::environment::GpuEnvironment;
use super::main_pipelines::{MainBindings, MainPipelines};
use super::mesh_cache::MeshCache;
use super::registries::NeutralTextures;
use super::{
    DrawKind, LIT_UNIFORM_BYTES, MAX_LIGHTS, PreparedBatch, PreparedScene, SceneAssets,
    ShadowDepthPass, bake_batches, lit_uniform_bytes, mat4_to_bytes,
};
use crate::{
    Buffer, BufferUsage, ColorFormat, CommandBuffer, Device, RenderPass as DriverRenderPass,
    SampledTexture, Sampler, Texture,
};

/// Owns all four main pipelines (colored/textured × lit/unlit) plus the
/// the shared view/lighting uniform, and the sampler for rendering a
/// [`Scene3d`] view. One instance per view (a UI overlay's `Scene2d` is
/// a separate, future-work sibling — see docs/graphics-design.md);
/// building it needs a real GPU device and the colour format of the
/// target it draws into (the HDR scene target, ADR 034). The emissive-extraction pipelines
/// [`crate::bloom::BloomPass`] draws through are *not* owned here — they
/// render into `BloomPass`'s own offscreen texture, whose format is
/// `BloomPass`'s to pick, not something this type should guess at (see
/// `bloom.rs`'s module doc and `uniform_buffer`).
#[derive(Debug)]
pub struct SceneRenderer {
    pub(crate) pipelines: MainPipelines,
    /// Meshes uploaded so far; see [`MeshCache`].
    pub(crate) meshes: MeshCache,
    pub(crate) uniform_buffer: Buffer,
    pub(crate) sampler: Sampler,
    /// A `Depth32Float` texture, both a render target (the shadow depth
    /// pass writes into it) and sampleable (the lit shaders' shadow
    /// lookup reads it) — see
    /// `rendering-wgpu::Device::create_shadow_map_texture`'s own doc
    /// comment for why [`Texture`] rather than the plain
    /// render-target-only `DepthTexture`.
    pub(crate) shadow_map: Texture,
    pub(crate) shadow_sampler: Sampler,
    /// What a draw binds for each map its material lacks.
    pub(crate) neutral: NeutralTextures,
    /// The environment lit surfaces are lit with (ADR 035), or a stand-in.
    pub(crate) environment: GpuEnvironment,
    shadow_depth: ShadowDepthPass,
}

/// [`SceneRenderer::new`]'s default shadow map resolution — generous
/// enough that both example scenes' shadows don't read as visibly
/// blocky, without the per-frame cost a much larger map would add. A
/// caller that wants sharper (or cheaper, blockier) shadows picks their
/// own value via [`SceneRenderer::with_shadow_map_size`] instead.
pub const DEFAULT_SHADOW_MAP_SIZE: u32 = 2048;

impl SceneRenderer {
    /// Builds a renderer drawing into targets of `format`, with
    /// [`DEFAULT_SHADOW_MAP_SIZE`] — see
    /// [`with_shadow_map_size`](Self::with_shadow_map_size) to pick a
    /// specific resolution (a larger map reduces blocky/aliased shadow
    /// edges at a real per-frame GPU cost; a smaller one is cheaper and
    /// blockier — there's no single right answer across every scene's
    /// scale and hardware budget, so this is an explicit, disclosed
    /// application choice, not a hardcoded constant with no escape
    /// hatch).
    #[must_use]
    pub fn new(device: &Device, format: ColorFormat) -> Self {
        Self::with_shadow_map_size(device, format, DEFAULT_SHADOW_MAP_SIZE)
    }

    #[must_use]
    pub fn with_shadow_map_size(
        device: &Device,
        format: ColorFormat,
        shadow_map_size: u32,
    ) -> Self {
        let shadow_map = device.create_shadow_map_texture(shadow_map_size);
        let shadow_sampler = device.create_comparison_sampler();
        let shadow_depth = ShadowDepthPass::new(device);

        // 416 bytes: see `lit_uniform_bytes`'s doc comment for the
        // layout. The unlit pipelines only read the first 64 (view_proj)
        // but share this same buffer — one write serves every pipeline.
        let uniform_buffer = device.create_buffer(LIT_UNIFORM_BYTES, BufferUsage::Uniform);
        let sampler = device.create_sampler();

        let environment = GpuEnvironment::none(device);
        let pipelines = MainPipelines::new(
            device,
            format,
            MainBindings {
                uniform_buffer: &uniform_buffer,
                shadow_map: &shadow_map,
                shadow_sampler: &shadow_sampler,
                environment: SampledTexture {
                    texture: &environment.texture,
                    sampler: &environment.sampler,
                },
            },
        );

        let renderer = Self {
            pipelines,
            meshes: MeshCache::default(),
            uniform_buffer,
            sampler,
            shadow_map,
            shadow_sampler,
            neutral: NeutralTextures::new(device),
            environment,
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
            meridian_diagnostics::log_warn!(
                "scene has {} lights, dropping {} beyond the cap of {MAX_LIGHTS}",
                view.lights.len(),
                view.lights.len() - MAX_LIGHTS
            );
        }
        device.write_buffer(
            &self.uniform_buffer,
            &lit_uniform_bytes(view, self.environment.uniform),
        );
    }

    /// Lights lit surfaces with `environment` from the next
    /// [`set_view`](Self::set_view) on (ADR 035); `None` returns to the
    /// scene's ambient colours. Uploads the map and rebuilds the bind groups
    /// that hold it.
    pub fn set_environment(&mut self, device: &Device, environment: Option<&EnvironmentMap>) {
        self.environment = match environment {
            Some(map) => GpuEnvironment::of(device, map),
            None => GpuEnvironment::none(device),
        };
        self.pipelines.rebind(
            device,
            MainBindings {
                uniform_buffer: &self.uniform_buffer,
                shadow_map: &self.shadow_map,
                shadow_sampler: &self.shadow_sampler,
                environment: SampledTexture {
                    texture: &self.environment.texture,
                    sampler: &self.environment.sampler,
                },
            },
        );
    }

    /// The shadow-casting depth pass: finds `scene`'s first
    /// `Light::Directional` (if any), builds its
    /// `directional_shadow_camera`, writes that light-space matrix
    /// into the (separate) shadow uniform buffer, bakes `scene` into
    /// instanced batches (reused afterward by [`draw`](Self::draw) — never
    /// rebuilt), and records a depth-only pass into the internal shadow
    /// map through `commands`. Returns the baked scene plus the
    /// [`ShadowLight`] for the caller to pass
    /// into [`set_view`](Self::set_view) before its own (separate, later)
    /// main color pass — see this module's own doc for why the shadow
    /// pass can't be folded into [`submit_scene3d`](super::submit_scene3d) itself (it has to
    /// finish, as its own pass, before the caller's main pass opens on
    /// the same `commands`).
    ///
    /// No `Light::Directional` in `scene.lights`: still records an
    /// (empty-of-occluders-mattering) depth pass for consistency, and
    /// returns `shadow_caster = u32::MAX`, which `shade`'s lookup never
    /// matches — every light renders fully lit, the same as before
    /// shadows existed.
    pub fn render_shadow_pass(
        &mut self,
        device: &Device,
        commands: &mut CommandBuffer<'_>,
        scene: &Scene3d,
        assets: &SceneAssets<'_>,
    ) -> PreparedScene {
        let shadow = ShadowLight::for_scene(scene);
        device.write_buffer(
            &self.shadow_depth.uniform_buffer,
            &mat4_to_bytes(shadow.view_proj),
        );
        let baked = bake_batches(device, self, scene, assets);
        self.record_shadow_casters(commands, &baked.batches, baked.instances.as_ref());
        PreparedScene {
            batches: baked.batches,
            instances: baked.instances,
            shadow,
        }
    }

    /// Records the depth-only pass of every shadow-casting batch.
    fn record_shadow_casters(
        &self,
        commands: &mut CommandBuffer<'_>,
        batches: &[PreparedBatch],
        instances: Option<&Buffer>,
    ) {
        let mut pass = commands.begin_shadow_pass(&self.shadow_map);
        let Some(instances) = instances else {
            return;
        };
        pass.set_pipeline(&self.shadow_depth.pipeline);
        pass.set_bind_group(0, &self.shadow_depth.bind_group);
        for batch in batches.iter().filter(|batch| batch.casts_shadow) {
            self.draw_batch(&mut pass, batch, instances);
        }
    }

    /// Records one draw call per batch against `pass`, in the scene's draw
    /// order (opaque, then translucent far to near, then on top), switching
    /// pipeline and bind group per batch's family, lighting and state —
    /// call inside the render pass, after
    /// [`render_shadow_pass`](Self::render_shadow_pass) built `prepared`
    /// and [`set_view`](Self::set_view) wrote this frame's uniform.
    pub fn draw(&self, pass: &mut DriverRenderPass<'_>, prepared: &PreparedScene) {
        let Some(instances) = &prepared.instances else {
            return;
        };
        for batch in &prepared.batches {
            let main = self.pipelines.get(batch.pipeline_key());
            pass.set_pipeline(&main.pipeline);
            match (&batch.kind, &main.shared_bind_group) {
                (DrawKind::Textured { bind_group }, _) | (DrawKind::Colored, Some(bind_group)) => {
                    pass.set_bind_group(0, bind_group);
                }
                (DrawKind::Colored, None) => continue,
            }
            self.draw_batch(pass, batch, instances);
        }
    }

    /// Binds `batch`'s mesh and the instance buffer, then draws its
    /// instances in one call. The pipeline and bind group are the caller's.
    pub(crate) fn draw_batch(
        &self,
        pass: &mut DriverRenderPass<'_>,
        batch: &PreparedBatch,
        instances: &Buffer,
    ) {
        let Some(mesh) = self.meshes.get(batch.mesh) else {
            return;
        };
        pass.set_vertex_buffer(0, &mesh.vertex_buffer);
        pass.set_vertex_buffer(1, instances);
        pass.set_index_buffer_u32(&mesh.index_buffer);
        pass.draw_indexed_instanced(0..mesh.index_count, batch.instances.clone());
    }
}
