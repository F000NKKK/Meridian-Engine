//! Pipelines and state that render a 3-D scene.

use core::ops::Range;

use meridian_rendering_domain::scene::Scene3d;
use meridian_rendering_domain::{EnvironmentMap, Frustum, LocalShadow, MeshRegistry, ShadowLight};

use super::batching::VisibleBatches;
use super::environment::GpuEnvironment;
use super::main_pipelines::MainPipelines;
use super::mesh_cache::MeshCache;
use super::prepared_view::PreparedView;
use super::registries::NeutralTextures;
use super::skinning::SkinPipeline;
use super::{
    ATLAS_COLUMNS, ATLAS_ROWS, CascadeCasters, DrawKind, PreparedBatch, PreparedScene, SceneAssets,
    ShadowDepthPass, bake_batches, face_tile, mat4_to_bytes, spot_tile, tile_rect,
};
use crate::{
    Buffer, ColorFormat, CommandBuffer, Device, RenderPass as DriverRenderPass, Sampler, Texture,
};

/// Owns the main pipelines and shared GPU assets. Camera and lighting
/// uniforms belong to each [`PreparedScene`], not to this renderer.
#[derive(Debug)]
pub struct SceneRenderer {
    pub(crate) pipelines: MainPipelines,
    /// Meshes uploaded so far; see [`MeshCache`].
    pub(crate) meshes: MeshCache,
    pub(crate) sampler: Sampler,
    /// The shadow atlas: a 2 x 2 grid of cascade tiles of
    /// [`tile_size`](Self::tile_size) texels, in one `Depth32Float` texture, both a render target (the shadow depth
    /// pass writes into it) and sampleable (the lit shaders' shadow
    /// lookup reads it) — see
    /// `rendering-wgpu::Device::create_shadow_map_texture`'s own doc
    /// comment for why [`Texture`] rather than the plain
    /// render-target-only `DepthTexture`.
    pub(crate) shadow_map: Texture,
    pub(crate) shadow_sampler: Sampler,
    /// Texels along one side of a cascade's tile in the shadow atlas.
    tile_size: u32,
    /// What a draw binds for each map its material lacks.
    pub(crate) neutral: NeutralTextures,
    /// The environment lit surfaces are lit with (ADR 035), or a stand-in.
    pub(crate) environment: GpuEnvironment,
    shadow_depth: ShadowDepthPass,
    /// Poses skinned meshes before the passes draw them (ADR 047).
    pub(crate) skinning: SkinPipeline,
}

/// [`SceneRenderer::new`]'s default shadow resolution: texels along one side
/// of a cascade's map (ADR 037). The four cascades together are a 2048-texel
/// atlas, the size the single map had before cascades. A caller that wants
/// sharper (or cheaper, blockier) shadows picks their own value via
/// [`SceneRenderer::with_shadow_map_size`] instead.
pub const DEFAULT_SHADOW_MAP_SIZE: u32 = 1024;

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

    /// Like [`new`](Self::new) with `shadow_map_size` texels along one side of
    /// each cascade's map; the atlas holding the four is twice that wide.
    #[must_use]
    pub fn with_shadow_map_size(
        device: &Device,
        format: ColorFormat,
        shadow_map_size: u32,
    ) -> Self {
        let shadow_map = device.create_shadow_map_texture(
            shadow_map_size * ATLAS_COLUMNS,
            shadow_map_size * ATLAS_ROWS,
        );
        let shadow_sampler = device.create_comparison_sampler();
        let shadow_depth = ShadowDepthPass::new(device);

        let sampler = device.create_sampler();
        let environment = GpuEnvironment::none(device);
        let pipelines = MainPipelines::new(device, format);
        let renderer = Self {
            pipelines,
            meshes: MeshCache::default(),
            sampler,
            shadow_map,
            shadow_sampler,
            tile_size: shadow_map_size,
            neutral: NeutralTextures::new(device),
            environment,
            shadow_depth,
            skinning: SkinPipeline::new(device),
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

    /// Lights lit surfaces with `environment` in subsequently prepared views.
    pub fn set_environment(&mut self, device: &Device, environment: Option<&EnvironmentMap>) {
        self.environment = match environment {
            Some(map) => GpuEnvironment::of(device, map),
            None => GpuEnvironment::none(device),
        };
    }

    /// The shadow-casting depth pass: finds `scene`'s first
    /// `Light::Directional` (if any), builds its
    /// `directional_shadow_camera`, writes that light-space matrix
    /// into the (separate) shadow uniform buffer, bakes `scene` into
    /// instanced batches (reused afterward by [`draw`](Self::draw) — never
    /// rebuilt), and records a depth-only pass into the internal shadow
    /// map through `commands`. Returns the baked scene with its own camera,
    /// lighting and shadow bindings for the later main colour pass — see
    /// this module's own doc for why the shadow
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
        let shadow = ShadowLight::for_scene(scene, self.tile_size);
        let prepared_view = PreparedView::new(device, self, scene, &shadow);
        let tiles = shadow_tiles(&shadow);
        for tile in &tiles {
            device.write_buffer(
                &self.shadow_depth.tiles[tile.index].buffer,
                &mat4_to_bytes(tile.view_proj),
            );
        }
        let frusta: Vec<Frustum> = tiles.iter().map(|tile| tile.frustum.clone()).collect();
        let baked = bake_batches(device, self, scene, assets, &prepared_view.uniform);
        for job in &baked.skin_jobs {
            commands.dispatch_compute(self.skinning.pipeline(), &job.bind_group, job.workgroups);
        }
        let casters = CascadeCasters::cull(&baked.batches, &baked.records, &baked.bounds, &frusta);
        let indices: Vec<usize> = tiles.iter().map(|tile| tile.index).collect();
        let draws = TileDraws {
            tiles: &indices,
            casters: &casters,
        };
        self.record_shadow_casters(commands, &baked.batches, &draws, casters.upload(device));
        let frustum = Frustum::from_view_projection(scene.camera.view_projection_matrix());
        let visible = VisibleBatches::cull(&baked.batches, &baked.records, &baked.bounds, &frustum);
        PreparedScene {
            instances: visible.upload(device),
            batches: visible.batches,
            shadow,
            view: prepared_view,
            has_emissive: scene
                .renderables
                .iter()
                .filter_map(|renderable| assets.materials.get(renderable.material))
                .any(|material| material.emissive.iter().any(|channel| *channel > 0.0)),
        }
    }

    /// Prepares an unshadowed secondary view without replacing the main shadow atlas.
    pub fn prepare_viewport(
        &mut self,
        device: &Device,
        commands: &mut CommandBuffer<'_>,
        scene: &Scene3d,
        assets: &SceneAssets<'_>,
    ) -> PreparedScene {
        let shadow = ShadowLight::for_scene(scene, self.tile_size);
        let prepared_view = PreparedView::new(device, self, scene, &shadow);
        let baked = bake_batches(device, self, scene, assets, &prepared_view.uniform);
        for job in &baked.skin_jobs {
            commands.dispatch_compute(self.skinning.pipeline(), &job.bind_group, job.workgroups);
        }
        let frustum = Frustum::from_view_projection(scene.camera.view_projection_matrix());
        let visible = VisibleBatches::cull(&baked.batches, &baked.records, &baked.bounds, &frustum);
        PreparedScene {
            instances: visible.upload(device),
            batches: visible.batches,
            shadow,
            view: prepared_view,
            has_emissive: scene
                .renderables
                .iter()
                .filter_map(|renderable| assets.materials.get(renderable.material))
                .any(|material| material.emissive.iter().any(|channel| *channel > 0.0)),
        }
    }

    /// Readies the renderer to bake a scene: drops the GPU copies of meshes
    /// that are no longer registered and starts the frame's skinned
    /// instances over (ADR 047).
    pub(crate) fn begin_bake(&mut self, registry: &MeshRegistry) {
        self.meshes.retain_registered(registry);
        self.skinning.begin_frame();
    }

    /// Records the depth-only pass: each cascade draws only the casters that
    /// reach its volume (ADR 040), from its own copy of their records.
    fn record_shadow_casters(
        &self,
        commands: &mut CommandBuffer<'_>,
        batches: &[PreparedBatch],
        draws: &TileDraws<'_>,
        records: Option<Buffer>,
    ) {
        let mut pass = commands.begin_shadow_pass(&self.shadow_map);
        let Some(records) = records else {
            return;
        };
        pass.set_pipeline(&self.shadow_depth.pipeline);
        for (&index, runs) in draws.tiles.iter().zip(&draws.casters.runs) {
            let rect = tile_rect(index, self.tile_size);
            pass.set_viewport(rect.x, rect.y, rect.size, rect.size);
            pass.set_bind_group(0, &self.shadow_depth.tiles[index].bind_group);
            for run in runs {
                self.draw_instances(
                    &mut pass,
                    &batches[run.batch],
                    &records,
                    run.instances.clone(),
                );
            }
        }
    }

    /// Records one draw call per batch against `pass`, in the scene's draw
    /// order, using the camera and lighting bindings owned by `prepared`.
    pub fn draw(&self, pass: &mut DriverRenderPass<'_>, prepared: &PreparedScene) {
        let Some(instances) = &prepared.instances else {
            return;
        };
        let mut bound: Option<&PreparedBatch> = None;
        for batch in &prepared.batches {
            if bound.is_none_or(|previous| !previous.binds_like(batch)) {
                let main = self.pipelines.get(batch.pipeline_key());
                pass.set_pipeline(&main.pipeline);
                match &batch.kind {
                    DrawKind::Textured { bind_group } => pass.set_bind_group(0, bind_group),
                    DrawKind::Colored => {
                        let Some(bind_group) = prepared.view.colored(batch.pipeline_key()) else {
                            continue;
                        };
                        pass.set_bind_group(0, bind_group);
                    }
                }
                bound = Some(batch);
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
        self.draw_instances(pass, batch, instances, batch.instances.clone());
    }

    /// [`draw_batch`](Self::draw_batch) for just the records `range` of
    /// `instances`.
    fn draw_instances(
        &self,
        pass: &mut DriverRenderPass<'_>,
        batch: &PreparedBatch,
        instances: &Buffer,
        range: Range<u32>,
    ) {
        let Some(mesh) = self.meshes.get(batch.mesh) else {
            return;
        };
        pass.set_vertex_buffer(0, batch.vertices.as_deref().unwrap_or(&mesh.vertex_buffer));
        pass.set_vertex_buffer(1, instances);
        pass.set_index_buffer_u32(&mesh.index_buffer);
        pass.draw_indexed_instanced(0..mesh.index_count, range);
    }
}

/// The atlas tile of each list of runs in `casters`, in the same order.
struct TileDraws<'a> {
    tiles: &'a [usize],
    casters: &'a CascadeCasters,
}

/// One shadow tile to draw: its place in the atlas, the light's view of it
/// and the volume casters are culled against.
struct ShadowTile {
    index: usize,
    view_proj: [[f32; 4]; 4],
    frustum: Frustum,
}

/// The tiles of `shadow` to draw: the four cascades, then the six faces of
/// each point light that casts one (ADR 043) and the one tile of each spot
/// light (ADR 045). A light keeps its slot's tiles, so the shader finds them
/// by slot whichever slots are empty.
fn shadow_tiles(shadow: &ShadowLight) -> Vec<ShadowTile> {
    let cascades = shadow
        .cascades
        .view_proj
        .into_iter()
        .zip(shadow.cascades.frusta());
    let mut tiles: Vec<ShadowTile> = cascades
        .enumerate()
        .map(|(index, (view_proj, frustum))| ShadowTile {
            index,
            view_proj,
            frustum,
        })
        .collect();
    for (slot, local) in shadow.locals.iter().enumerate() {
        match local {
            Some(LocalShadow::Point(point)) => {
                let faces = point.view_proj.into_iter().zip(point.frusta());
                tiles.extend(
                    faces
                        .enumerate()
                        .map(|(face, (view_proj, frustum))| ShadowTile {
                            index: face_tile(slot, face),
                            view_proj,
                            frustum,
                        }),
                );
            }
            Some(LocalShadow::Spot(spot)) => tiles.push(ShadowTile {
                index: spot_tile(slot),
                view_proj: spot.view_proj,
                frustum: spot.frustum(),
            }),
            None => {}
        }
    }
    tiles
}
