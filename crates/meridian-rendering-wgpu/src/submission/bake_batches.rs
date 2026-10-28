//! Turning a scene's renderables into instanced batches.

use meridian_gac::{Motor3, Vec3};
use meridian_rendering_domain::scene::{Material, Renderable3d, Scene3d};
use meridian_rendering_domain::{
    BatchKey, DrawItem, DrawState, MeshHandle, MeshSource, TextureHandle, plan_batches,
};

use super::instance_data::push_instance;
use super::main_pipelines::PipelineKey;
use super::{DrawKind, PreparedBatch, SceneAssets, SceneRenderer};
use crate::{Buffer, BufferUsage, Device, LitTextures, SampledTexture, Texture};

/// A renderable whose mesh and material resolved.
struct Resolved<'a> {
    mesh: MeshHandle,
    source: &'a MeshSource,
    material: &'a Material,
    frame: &'a Motor3,
    /// The albedo texture, when the material names one that resolved.
    albedo: Option<(TextureHandle, &'a Texture)>,
    /// The metallic-roughness map, when the material names one that resolved.
    surface_map: Option<(TextureHandle, &'a Texture)>,
    casts_shadow: bool,
}

impl<'a> Resolved<'a> {
    /// `None` when the mesh or material is not registered: an asset that
    /// has not loaded yet, not a bug to panic over.
    fn of(renderable: &'a Renderable3d, assets: &SceneAssets<'a>) -> Option<Self> {
        let source = assets.meshes.get(renderable.mesh)?;
        let material = assets.materials.get(renderable.material)?;
        let albedo = material
            .albedo
            .and_then(|handle| assets.textures.get(handle).map(|texture| (handle, texture)));
        let surface_map = material
            .metallic_roughness
            .and_then(|handle| assets.textures.get(handle).map(|texture| (handle, texture)));
        Some(Self {
            mesh: renderable.mesh,
            source,
            material,
            frame: &renderable.frame,
            albedo,
            surface_map,
            casts_shadow: renderable.casts_shadow,
        })
    }

    fn item(&self, eye: Vec3) -> DrawItem {
        let origin = self.frame.transform_point(Vec3::ZERO);
        DrawItem {
            key: BatchKey {
                mesh: self.mesh,
                albedo: self.albedo.map(|(handle, _)| handle),
                metallic_roughness: self.surface_map.map(|(handle, _)| handle),
                lit: !self.material.unlit,
                state: DrawState::of(self.material),
                casts_shadow: self.casts_shadow,
            },
            distance_squared: (origin - eye).length_squared(),
        }
    }
}

/// The batches of one scene and the instance buffer they index.
pub(crate) struct BakedBatches {
    pub batches: Vec<PreparedBatch>,
    pub instances: Option<Buffer>,
}

/// Resolves `scene`'s renderables, uploads any mesh not yet on the GPU,
/// groups the renderables into batches in drawing order and uploads their
/// instance records in one buffer.
pub(crate) fn bake_batches(
    device: &Device,
    renderer: &mut SceneRenderer,
    scene: &Scene3d,
    assets: &SceneAssets<'_>,
) -> BakedBatches {
    renderer.meshes.retain_registered(assets.meshes);
    let eye = scene.camera.frame.transform_point(Vec3::ZERO);
    let resolved: Vec<Resolved<'_>> = scene
        .renderables
        .iter()
        .filter_map(|renderable| Resolved::of(renderable, assets))
        .collect();
    let items: Vec<DrawItem> = resolved.iter().map(|r| r.item(eye)).collect();

    let mut bytes = Vec::new();
    let mut batches = Vec::new();
    let mut first = 0u32;
    for planned in plan_batches(&items) {
        let lead = &resolved[planned.members[0]];
        renderer.meshes.ensure(device, lead.mesh, lead.source);
        for &member in &planned.members {
            let instance = &resolved[member];
            push_instance(&mut bytes, instance.frame, instance.material);
        }
        let count = u32::try_from(planned.members.len()).unwrap_or(u32::MAX);
        batches.push(PreparedBatch {
            mesh: lead.mesh,
            kind: draw_kind(device, renderer, lead),
            lit: planned.key.lit,
            state: planned.key.state,
            casts_shadow: planned.key.casts_shadow,
            instances: first..first.saturating_add(count),
        });
        first = first.saturating_add(count);
    }
    BakedBatches {
        batches,
        instances: upload_instances(device, &bytes),
    }
}

fn draw_kind(device: &Device, renderer: &SceneRenderer, lead: &Resolved<'_>) -> DrawKind {
    if lead.albedo.is_none() && lead.surface_map.is_none() {
        return DrawKind::Colored;
    }
    let key = PipelineKey {
        textured: true,
        lit: !lead.material.unlit,
        state: DrawState::of(lead.material),
    };
    let albedo = lead
        .albedo
        .map_or(&renderer.white_albedo, |(_, texture)| texture);
    let surface_map = lead
        .surface_map
        .map_or(&renderer.white_surface, |(_, texture)| texture);
    DrawKind::Textured {
        bind_group: textured_bind_group(device, renderer, [albedo, surface_map], key),
    }
}

fn upload_instances(device: &Device, bytes: &[u8]) -> Option<Buffer> {
    if bytes.is_empty() {
        return None;
    }
    let buffer = device.create_buffer(bytes.len(), BufferUsage::Vertex);
    device.write_buffer(&buffer, bytes);
    Some(buffer)
}

/// A fresh bind group naming the `[albedo, metallic_roughness]` textures,
/// made against the pipeline `key` selects (a bind group only fits its own
/// pipeline). Lit textured draws need the shadow map and sampler too and
/// the metallic-roughness map (see `lit_textured_shader_wgsl`'s bindings 3
/// to 5); unlit draws ignore lighting, so theirs stays the plain three
/// bindings.
fn textured_bind_group(
    device: &Device,
    renderer: &SceneRenderer,
    [albedo, surface_map]: [&Texture; 2],
    key: PipelineKey,
) -> crate::BindGroup {
    let pipeline = &renderer.pipelines.get(key).pipeline;
    if key.lit {
        device.create_textured_shadow_bind_group(
            pipeline,
            &renderer.uniform_buffer,
            LitTextures {
                albedo: SampledTexture {
                    texture: albedo,
                    sampler: &renderer.sampler,
                },
                metallic_roughness: surface_map,
                shadow: SampledTexture {
                    texture: &renderer.shadow_map,
                    sampler: &renderer.shadow_sampler,
                },
            },
        )
    } else {
        device.create_textured_bind_group(
            pipeline,
            &renderer.uniform_buffer,
            albedo,
            &renderer.sampler,
        )
    }
}
