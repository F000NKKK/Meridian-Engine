//! The submission bridge: turns a culled [`Scene3D`] into real GPU draws
//! through `graphics-driver` — step 2 of docs/graphics-design.md's scene
//! vocabulary implementation order. This is the one place in
//! `graphics-core` allowed to touch `graphics-driver` objects; every
//! other type in this crate stays driver-free.
//!
//! **Mesh/material/texture identity is handle-based, never a raw index
//! or a driver object** ([`MeshRegistry`]/[`MaterialRegistry`]/
//! [`TextureRegistry`] wrap `memory-core::ResourcePool`, resolved by
//! [`MeshHandle`]/[`MaterialHandle`]/[`TextureHandle`] — see ADR 002).
//!
//! **Two pipelines, chosen per renderable at bake time:** a material
//! with a [`TextureHandle`] that resolves in the [`TextureRegistry`]
//! draws through [`TEXTURED_SHADER_WGSL`] (samples the albedo, tinted by
//! `base_color_factor`); every other material draws through
//! [`UNLIT_SHADER_WGSL`] (flat `base_color_factor`, no texture). Neither
//! does real lighting yet — both are unlit; Blinn-Phong lighting is the
//! next design-doc step, layered onto these same two pipelines rather
//! than replacing them.
//!
//! **No per-instance uniform, no GPU-side instancing yet.** A render
//! pass here has exactly one shared uniform buffer (the view-projection
//! matrix) per view — there is no dynamic-offset or per-draw uniform
//! mechanism in `graphics-driver` today, and writing a *shared* uniform
//! once per draw would race (every draw only executes once the whole
//! command buffer is submitted, by which point the last write wins for
//! *all* of them — the same footgun the soft-body examples' buffer-
//! lifetime comments call out). So each [`Renderable3D`]'s world-space
//! position, its UV (if textured) and its material's tint are baked into
//! a fresh vertex buffer at [`SceneRenderer::prepare`] time, once per
//! instance, per frame — correct today, and exactly the CPU-side cost
//! docs/graphics-design.md's "GPU-driven rendering" section already
//! names as the long-term thing to fix via instancing/indirect draw.
//! Per-textured-draw bind groups are similarly rebuilt every frame
//! rather than cached per texture — the same "correct now, batch later"
//! trade-off.

use meridian_gac_core::{Motor3, Vec3};
use meridian_graphics_driver::{
    BindGroup, Buffer, BufferUsage, Device, RenderPass as DriverRenderPass, RenderPipeline,
    Sampler, Surface, Texture, VertexAttributeDesc, VertexFormat, VertexLayout,
};
use meridian_memory_core::ResourcePool;

use crate::scene::{Material, Renderable3D, Scene3D};
use crate::{Camera, MaterialHandle, MeshHandle, TextureHandle};

/// A flat, unlit pipeline: `view_proj * vec4(position, 1)`, output color
/// is the per-vertex color baked in at [`SceneRenderer::prepare`] time
/// (see the module doc for why it's per-vertex, not a material uniform).
/// Used for materials with no resolvable albedo texture.
pub const UNLIT_SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// An unlit, textured pipeline: samples `albedo_tex` and multiplies by
/// the per-vertex tint (the material's `base_color_factor`, baked in at
/// [`SceneRenderer::prepare`] time — see the module doc for why tint
/// travels per-vertex rather than through a second uniform). Used for
/// materials whose albedo resolves in a [`TextureRegistry`].
pub const TEXTURED_SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;
@group(0) @binding(1)
var albedo_tex: texture_2d<f32>;
@group(0) @binding(2)
var albedo_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) tint: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.tint = in.tint;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(albedo_tex, albedo_sampler, in.uv) * in.tint;
}
"#;

/// [`UNLIT_SHADER_WGSL`]'s vertex layout: `position` then `color`,
/// tightly packed.
fn colored_vertex_layout() -> VertexLayout {
    VertexLayout {
        stride: 28, // 3 + 4 = 7 floats
        attributes: vec![
            VertexAttributeDesc {
                location: 0,
                format: VertexFormat::Float32x3,
                offset: 0,
            },
            VertexAttributeDesc {
                location: 1,
                format: VertexFormat::Float32x4,
                offset: 12,
            },
        ],
    }
}

/// [`TEXTURED_SHADER_WGSL`]'s vertex layout: `position`, `uv`, `tint`,
/// tightly packed.
fn textured_vertex_layout() -> VertexLayout {
    VertexLayout {
        stride: 36, // 3 + 2 + 4 = 9 floats
        attributes: vec![
            VertexAttributeDesc {
                location: 0,
                format: VertexFormat::Float32x3,
                offset: 0,
            },
            VertexAttributeDesc {
                location: 1,
                format: VertexFormat::Float32x2,
                offset: 12,
            },
            VertexAttributeDesc {
                location: 2,
                format: VertexFormat::Float32x4,
                offset: 20,
            },
        ],
    }
}

fn mat4_to_bytes(m: [[f32; 4]; 4]) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    let mut offset = 0;
    for column in m {
        for component in column {
            bytes[offset..offset + 4].copy_from_slice(&component.to_le_bytes());
            offset += 4;
        }
    }
    bytes
}

/// The CPU-side source a [`MeshHandle`] resolves to: local-space
/// positions, per-vertex UVs and triangle indices, decoded elsewhere
/// (typically `asset-core::ObjDecoder`/glTF once that lands) and
/// registered here once. `uvs.len()` must equal `positions.len()`,
/// `indices.len()` must be a multiple of 3 (triangle list), and
/// `positions.len()` must fit in `u16` — `graphics-driver` only supports
/// `u16` index buffers today (see
/// `meridian_graphics_driver::RenderPass::set_index_buffer_u16`'s own
/// doc comment); meshes beyond 65535 vertices are a future `u32`-index
/// follow-up, not something this bridge silently mishandles (see
/// [`MeshRegistry::register`]).
#[derive(Debug, Clone)]
pub struct MeshSource {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

/// Why [`MeshRegistry::register`] refused a [`MeshSource`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshRegistryError {
    /// More vertices than a `u16` index can address (see [`MeshSource`]'s
    /// doc comment).
    TooManyVertices { count: usize },
    /// `indices.len()` isn't a multiple of 3 — not a triangle list.
    NotATriangleList { index_count: usize },
    /// `uvs.len()` doesn't match `positions.len()` — every vertex needs
    /// exactly one UV, even on meshes only ever drawn unlit/untextured
    /// (the registry doesn't know a mesh's eventual material up front).
    UvCountMismatch { positions: usize, uvs: usize },
}

impl core::fmt::Display for MeshRegistryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MeshRegistryError::TooManyVertices { count } => {
                write!(
                    f,
                    "mesh has {count} vertices, beyond u16 index range (65535)"
                )
            }
            MeshRegistryError::NotATriangleList { index_count } => {
                write!(f, "index count {index_count} is not a multiple of 3")
            }
            MeshRegistryError::UvCountMismatch { positions, uvs } => {
                write!(f, "{positions} positions but {uvs} uvs — must match")
            }
        }
    }
}

impl std::error::Error for MeshRegistryError {}

impl meridian_foundation::EngineError for MeshRegistryError {}

/// Handle-addressed storage for [`MeshSource`]s — the CPU-side registry
/// [`Renderable3D::mesh`] resolves against. Owns no GPU objects; GPU
/// buffers are built fresh per instance in [`SceneRenderer::prepare`]
/// (see the module doc).
#[derive(Debug, Default)]
pub struct MeshRegistry {
    sources: ResourcePool<MeshSource>,
}

impl MeshRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, source: MeshSource) -> Result<MeshHandle, MeshRegistryError> {
        if source.positions.len() > u16::MAX as usize {
            return Err(MeshRegistryError::TooManyVertices {
                count: source.positions.len(),
            });
        }
        if !source.indices.len().is_multiple_of(3) {
            return Err(MeshRegistryError::NotATriangleList {
                index_count: source.indices.len(),
            });
        }
        if source.uvs.len() != source.positions.len() {
            return Err(MeshRegistryError::UvCountMismatch {
                positions: source.positions.len(),
                uvs: source.uvs.len(),
            });
        }
        Ok(MeshHandle::new(self.sources.insert(source)))
    }

    pub fn unregister(&mut self, handle: MeshHandle) -> Option<MeshSource> {
        self.sources.remove(handle.handle)
    }

    pub fn get(&self, handle: MeshHandle) -> Option<&MeshSource> {
        self.sources.get(handle.handle)
    }
}

/// Handle-addressed storage for [`Material`]s. Materials are plain data
/// (see `scene::Material`'s own doc comment); this registry exists so
/// [`Renderable3D::material`] is a stable handle rather than an inline
/// value, matching every other resource identity in the workspace.
#[derive(Debug, Default)]
pub struct MaterialRegistry {
    materials: ResourcePool<Material>,
}

impl MaterialRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, material: Material) -> MaterialHandle {
        MaterialHandle::new(self.materials.insert(material))
    }

    pub fn unregister(&mut self, handle: MaterialHandle) -> Option<Material> {
        self.materials.remove(handle.handle)
    }

    pub fn get(&self, handle: MaterialHandle) -> Option<&Material> {
        self.materials.get(handle.handle)
    }
}

/// GPU-resident textures, handle-addressed by [`TextureHandle`] — the
/// registry a [`Material::albedo`] resolves against. Upload happens once
/// via [`upload`](Self::upload), from an already-decoded
/// `asset-core::ImageData` (RGBA8) — this bridge does not decode image
/// bytes itself, matching the crate's existing "decoding belongs to
/// `asset-core`" boundary.
#[derive(Debug, Default)]
pub struct TextureRegistry {
    textures: ResourcePool<Texture>,
}

impl TextureRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Uploads `image`'s pixels to a new GPU texture and returns its
    /// handle.
    pub fn upload(
        &mut self,
        device: &Device,
        image: &meridian_asset_core::ImageData,
    ) -> TextureHandle {
        let texture = device.create_texture_2d(image.width, image.height);
        device.write_texture(&texture, &image.pixels);
        TextureHandle::new(self.textures.insert(texture))
    }

    pub fn unregister(&mut self, handle: TextureHandle) -> Option<Texture> {
        self.textures.remove(handle.handle)
    }

    pub fn get(&self, handle: TextureHandle) -> Option<&Texture> {
        self.textures.get(handle.handle)
    }
}

/// Which pipeline one [`DrawBuffers`] entry draws through, and the
/// per-draw state that pipeline needs beyond the shared vertex/index
/// buffers.
enum DrawKind {
    Colored,
    /// A fresh bind group naming this draw's specific texture — see the
    /// module doc's "no per-texture bind-group caching yet" note.
    Textured {
        bind_group: BindGroup,
    },
}

/// One renderable's baked-for-this-frame GPU state — built by
/// [`SceneRenderer::prepare`] *before* the render pass opens, kept alive
/// by the caller until [`SceneRenderer::draw`] (and the pass itself) is
/// done with them. This split exists because `RenderPass::set_vertex_buffer`
/// borrows its buffer for the pass's lifetime — the same reason
/// `soft_body_rubber_balls`'s own `ball_buffers` are built up front (see
/// that example's module doc).
pub struct DrawBuffers {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
    kind: DrawKind,
}

/// Transforms `source`'s local-space positions by `frame`, bakes in
/// `material`'s tint (and UVs, for the textured path), and uploads the
/// result — the flat/unlit path when `albedo` is `None`, the textured
/// path (with a bind group naming `albedo`) otherwise.
fn bake_draw_buffers(
    device: &Device,
    renderer: &SceneRenderer,
    source: &MeshSource,
    material: &Material,
    frame: &Motor3,
    albedo: Option<&Texture>,
) -> DrawBuffers {
    let index_bytes: Vec<u8> = source
        .indices
        .iter()
        .flat_map(|&i| (i as u16).to_le_bytes())
        .collect();
    let index_buffer = device.create_buffer(index_bytes.len(), BufferUsage::Index);
    device.write_buffer(&index_buffer, &index_bytes);
    let index_count = source.indices.len() as u32;

    match albedo {
        Some(texture) => {
            let mut vertex_bytes = Vec::with_capacity(source.positions.len() * 36);
            for (&[x, y, z], &[u, v]) in source.positions.iter().zip(&source.uvs) {
                let world = frame.transform_point(Vec3::new(x, y, z));
                vertex_bytes.extend_from_slice(&world.x.to_le_bytes());
                vertex_bytes.extend_from_slice(&world.y.to_le_bytes());
                vertex_bytes.extend_from_slice(&world.z.to_le_bytes());
                vertex_bytes.extend_from_slice(&u.to_le_bytes());
                vertex_bytes.extend_from_slice(&v.to_le_bytes());
                for component in material.base_color_factor {
                    vertex_bytes.extend_from_slice(&component.to_le_bytes());
                }
            }
            let vertex_buffer = device.create_buffer(vertex_bytes.len(), BufferUsage::Vertex);
            device.write_buffer(&vertex_buffer, &vertex_bytes);
            let bind_group = device.create_textured_bind_group(
                &renderer.textured_pipeline,
                &renderer.uniform_buffer,
                texture,
                &renderer.sampler,
            );
            DrawBuffers {
                vertex_buffer,
                index_buffer,
                index_count,
                kind: DrawKind::Textured { bind_group },
            }
        }
        None => {
            let mut vertex_bytes = Vec::with_capacity(source.positions.len() * 28);
            for &[x, y, z] in &source.positions {
                let world = frame.transform_point(Vec3::new(x, y, z));
                vertex_bytes.extend_from_slice(&world.x.to_le_bytes());
                vertex_bytes.extend_from_slice(&world.y.to_le_bytes());
                vertex_bytes.extend_from_slice(&world.z.to_le_bytes());
                for component in material.base_color_factor {
                    vertex_bytes.extend_from_slice(&component.to_le_bytes());
                }
            }
            let vertex_buffer = device.create_buffer(vertex_bytes.len(), BufferUsage::Vertex);
            device.write_buffer(&vertex_buffer, &vertex_bytes);
            DrawBuffers {
                vertex_buffer,
                index_buffer,
                index_count,
                kind: DrawKind::Colored,
            }
        }
    }
}

/// Owns both pipelines (flat-color and textured) plus the shared camera
/// uniform and sampler for rendering a [`Scene3D`] view. One instance
/// per view (a UI overlay's `Scene2D` is a separate, future-work sibling
/// — see docs/graphics-design.md); building it needs a real GPU device
/// and a surface to match color/depth formats against.
pub struct SceneRenderer {
    colored_pipeline: RenderPipeline,
    colored_bind_group: BindGroup,
    textured_pipeline: RenderPipeline,
    uniform_buffer: Buffer,
    sampler: Sampler,
}

impl SceneRenderer {
    pub fn new(device: &Device, surface: &Surface) -> Self {
        let colored_shader = device.create_shader("meridian-unlit-colored", UNLIT_SHADER_WGSL);
        let colored_pipeline = device.create_render_pipeline(
            &colored_shader,
            "vs_main",
            "fs_main",
            &colored_vertex_layout(),
            surface,
            true,
        );
        let textured_shader = device.create_shader("meridian-unlit-textured", TEXTURED_SHADER_WGSL);
        let textured_pipeline = device.create_render_pipeline(
            &textured_shader,
            "vs_main",
            "fs_main",
            &textured_vertex_layout(),
            surface,
            true,
        );

        let uniform_buffer = device.create_buffer(64, BufferUsage::Uniform);
        let colored_bind_group =
            device.create_uniform_bind_group(&colored_pipeline, &uniform_buffer);
        let sampler = device.create_sampler();

        Self {
            colored_pipeline,
            colored_bind_group,
            textured_pipeline,
            uniform_buffer,
            sampler,
        }
    }

    /// Writes this view's view-projection matrix — call once per frame
    /// before [`draw`](Self::draw), not per draw call (see the module
    /// doc: this bridge has exactly one shared uniform per view).
    pub fn set_camera(&self, device: &Device, camera: &Camera) {
        device.write_buffer(
            &self.uniform_buffer,
            &mat4_to_bytes(camera.view_projection_matrix()),
        );
    }

    /// Builds one [`DrawBuffers`] per renderable whose mesh/material both
    /// resolve (a renderable naming a missing handle is silently skipped
    /// — the caller controls registry contents, so a miss here means an
    /// asset hasn't loaded yet, not a bug to panic over). Call this
    /// *before* opening the render pass; pass the result to
    /// [`draw`](Self::draw) once inside it.
    pub fn prepare(
        &self,
        device: &Device,
        renderables: &[Renderable3D],
        meshes: &MeshRegistry,
        materials: &MaterialRegistry,
        textures: &TextureRegistry,
    ) -> Vec<DrawBuffers> {
        renderables
            .iter()
            .filter_map(|renderable| {
                let source = meshes.get(renderable.mesh)?;
                let material = materials.get(renderable.material)?;
                let albedo = material.albedo.and_then(|handle| textures.get(handle));
                Some(bake_draw_buffers(
                    device,
                    self,
                    source,
                    material,
                    &renderable.frame,
                    albedo,
                ))
            })
            .collect()
    }

    /// Records one draw call per `buffers` entry against `pass`,
    /// switching pipeline/bind group per entry's baked
    /// baked draw kind — call inside the render pass, after
    /// [`prepare`](Self::prepare) built `buffers` and
    /// [`set_camera`](Self::set_camera) wrote this frame's matrix.
    pub fn draw(&self, pass: &mut DriverRenderPass<'_>, buffers: &[DrawBuffers]) {
        for entry in buffers {
            match &entry.kind {
                DrawKind::Colored => {
                    pass.set_pipeline(&self.colored_pipeline);
                    pass.set_bind_group(0, &self.colored_bind_group);
                }
                DrawKind::Textured { bind_group } => {
                    pass.set_pipeline(&self.textured_pipeline);
                    pass.set_bind_group(0, bind_group);
                }
            }
            pass.set_vertex_buffer(0, &entry.vertex_buffer);
            pass.set_index_buffer_u16(&entry.index_buffer);
            pass.draw_indexed(0..entry.index_count);
        }
    }
}

/// The whole per-frame path for a `Scene3D` view in one call: bake draw
/// buffers for the (already-culled) `scene.renderables`, write the
/// camera uniform, and record the draws — everything
/// [`SceneRenderer::prepare`] requires to happen before the pass, plus
/// [`SceneRenderer::draw`] inside it. Returns the buffers so the caller
/// keeps them alive until the surrounding `CommandBuffer::submit()` —
/// see [`DrawBuffers`]'s doc comment for why they can't just be dropped
/// here.
pub fn submit_scene3d(
    device: &Device,
    renderer: &SceneRenderer,
    pass: &mut DriverRenderPass<'_>,
    scene: &Scene3D,
    meshes: &MeshRegistry,
    materials: &MaterialRegistry,
    textures: &TextureRegistry,
) -> Vec<DrawBuffers> {
    renderer.set_camera(device, &scene.camera);
    let buffers = renderer.prepare(device, &scene.renderables, meshes, materials, textures);
    renderer.draw(pass, &buffers);
    buffers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> MeshSource {
        MeshSource {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            indices: vec![0, 1, 2],
        }
    }

    #[test]
    fn mesh_registry_rejects_non_triangle_index_counts() {
        let mut registry = MeshRegistry::new();
        let mut source = triangle();
        source.indices = vec![0, 1]; // not a multiple of 3
        assert_eq!(
            registry.register(source),
            Err(MeshRegistryError::NotATriangleList { index_count: 2 })
        );
    }

    #[test]
    fn mesh_registry_rejects_meshes_beyond_u16_vertex_range() {
        let mut registry = MeshRegistry::new();
        let count = u16::MAX as usize + 1;
        let source = MeshSource {
            positions: vec![[0.0, 0.0, 0.0]; count],
            uvs: vec![[0.0, 0.0]; count],
            indices: vec![0, 1, 2],
        };
        assert_eq!(
            registry.register(source),
            Err(MeshRegistryError::TooManyVertices { count })
        );
    }

    #[test]
    fn mesh_registry_rejects_uv_count_mismatch() {
        let mut registry = MeshRegistry::new();
        let mut source = triangle();
        source.uvs.pop();
        assert_eq!(
            registry.register(source),
            Err(MeshRegistryError::UvCountMismatch {
                positions: 3,
                uvs: 2
            })
        );
    }

    #[test]
    fn mesh_registry_round_trips_a_valid_mesh() {
        let mut registry = MeshRegistry::new();
        let handle = registry.register(triangle()).unwrap();
        assert_eq!(registry.get(handle).unwrap().indices, vec![0, 1, 2]);
        assert!(registry.unregister(handle).is_some());
        assert!(registry.get(handle).is_none());
    }

    #[test]
    fn material_registry_round_trips() {
        let mut registry = MaterialRegistry::new();
        let material = Material {
            base_color_factor: [1.0, 0.0, 0.0, 1.0],
            ..Default::default()
        };
        let handle = registry.register(material);
        assert_eq!(
            registry.get(handle).unwrap().base_color_factor,
            [1.0, 0.0, 0.0, 1.0]
        );
        assert!(registry.unregister(handle).is_some());
        assert!(registry.get(handle).is_none());
    }

    #[test]
    fn mat4_to_bytes_is_column_major_little_endian() {
        let m = [
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ];
        let bytes = mat4_to_bytes(m);
        assert_eq!(&bytes[0..4], &1.0f32.to_le_bytes());
        assert_eq!(&bytes[16..20], &5.0f32.to_le_bytes());
        assert_eq!(&bytes[60..64], &16.0f32.to_le_bytes());
    }

    /// `TextureRegistry::upload` against a real (or skipped) headless GPU
    /// device — the one piece of this module testable without a window
    /// (`SceneRenderer::new`/`prepare`/`draw` need a real `Surface`,
    /// which needs a window; those are exercised end-to-end by a
    /// windowed example instead, the same convention
    /// `meridian-graphics-driver`'s own module doc states for its
    /// windowed-only paths).
    #[tokio::test]
    async fn texture_registry_round_trips_a_real_upload() {
        let Ok(device) = Device::new().await else {
            eprintln!("skipping: no GPU device available");
            return;
        };
        let mut textures = TextureRegistry::new();
        let image = meridian_asset_core::ImageData {
            width: 1,
            height: 1,
            pixels: vec![255, 255, 255, 255],
        };
        let handle = textures.upload(&device, &image);
        assert!(textures.get(handle).is_some());
        assert!(textures.unregister(handle).is_some());
        assert!(textures.get(handle).is_none());
    }
}
