//! The submission bridge: turns a culled [`Scene3D`] into real GPU draws
//! through `graphics-driver` — step 2 of docs/graphics-design.md's scene
//! vocabulary implementation order. This is the one place in
//! `graphics-core` allowed to touch `graphics-driver` objects; every
//! other type in this crate stays driver-free.
//!
//! **Mesh/material identity is handle-based, never a raw index or a
//! driver object** ([`MeshRegistry`]/[`MaterialRegistry`] wrap
//! `memory-core::ResourcePool`, resolved by [`MeshHandle`]/
//! [`MaterialHandle`] — see ADR 002).
//!
//! **Current shading model — disclosed limitation, not hidden:**
//! `graphics-driver`'s render pipeline has one uniform-buffer bind group
//! and no texture/sampler bind-group support yet (see
//! `meridian-gpu-driver::Device::create_bind_group`'s buffers-only
//! signature), so every material renders as a flat, unlit
//! `base_color_factor` — no albedo texture sampling, no lighting. Adding
//! texture-sampled materials needs a `graphics-driver`/`gpu-driver`
//! extension (texture + sampler bind-group support), tracked as the next
//! follow-up in docs/roadmap.md; it does not change this bridge's shape,
//! only [`UNLIT_SHADER_WGSL`] and the vertex layout below it.
//!
//! **No per-instance uniform, no GPU-side instancing yet.** A render
//! pass here has exactly one uniform buffer (the view-projection
//! matrix), shared by every draw in the pass — there is no dynamic
//! offset or per-draw uniform mechanism in `graphics-driver` today, and
//! writing a *shared* uniform buffer once per draw would race (every
//! draw only executes once the whole command buffer is submitted, by
//! which point the last write wins for *all* of them — the same
//! footgun `RenderPass::begin_render_pass`'s buffer-lifetime comment in
//! the soft-body examples calls out for buffers). So each
//! [`Renderable3D`]'s world-space position *and* its material's color
//! are baked into a fresh vertex buffer at
//! [`prepare_draws`] time, once per instance, per frame — correct today,
//! and exactly the CPU-side cost docs/graphics-design.md's "GPU-driven
//! rendering" section already names as the long-term thing to fix via
//! instancing/indirect draw.

use meridian_gac_core::{Motor3, Vec3};
use meridian_graphics_driver::{
    BindGroup, Buffer, BufferUsage, Device, RenderPass as DriverRenderPass, RenderPipeline,
    Surface, VertexAttributeDesc, VertexFormat, VertexLayout,
};
use meridian_memory_core::ResourcePool;

use crate::scene::{Material, Renderable3D, Scene3D};
use crate::{Camera, MaterialHandle, MeshHandle};

/// A minimal unlit pipeline: `view_proj * vec4(position, 1)`, output
/// color is the per-vertex color baked in at [`prepare_draws`] time (see
/// the module doc for why it's per-vertex, not a material uniform).
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

/// [`UNLIT_SHADER_WGSL`]'s vertex layout: `position` then `color`, tightly
/// packed.
fn unlit_vertex_layout() -> VertexLayout {
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
/// positions and triangle indices, decoded elsewhere (typically
/// `asset-core::ObjDecoder`/glTF once that lands) and registered here
/// once. `indices.len()` must be a multiple of 3 (triangle list) and
/// `positions.len()` must fit in `u16` per-mesh — `graphics-driver` only
/// supports `u16` index buffers today (see
/// `meridian_graphics_driver::RenderPass::set_index_buffer_u16`'s own
/// doc comment); meshes beyond 65535 vertices are a future `u32`-index
/// follow-up, not something this bridge silently mishandles (see
/// [`MeshRegistry::register`]).
#[derive(Debug, Clone)]
pub struct MeshSource {
    pub positions: Vec<[f32; 3]>,
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
        }
    }
}

impl std::error::Error for MeshRegistryError {}

impl meridian_foundation::EngineError for MeshRegistryError {}

/// Handle-addressed storage for [`MeshSource`]s — the CPU-side registry
/// [`Renderable3D::mesh`] resolves against. Owns no GPU objects; GPU
/// buffers are built fresh per instance in [`prepare_draws`] (see the
/// module doc).
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

/// One renderable's baked-for-this-frame GPU buffers — built by
/// [`prepare_draws`] *before* the render pass opens, kept alive by the
/// caller until [`draw`] (and the pass itself) is done with them. This
/// split exists because `RenderPass::set_vertex_buffer` borrows its
/// buffer for the pass's lifetime — the same reason
/// `soft_body_rubber_balls`'s own `ball_buffers` are built up front (see
/// that example's module doc).
pub struct DrawBuffers {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
}

/// Builds one [`DrawBuffers`] per renderable whose mesh/material both
/// resolve (a renderable naming a missing handle is silently skipped —
/// the caller controls registry contents, so a miss here means an asset
/// hasn't loaded yet, not a bug to panic over). Call this *before*
/// opening the render pass; pass the result to [`draw`] once inside it.
pub fn prepare_draws(
    device: &Device,
    renderables: &[Renderable3D],
    meshes: &MeshRegistry,
    materials: &MaterialRegistry,
) -> Vec<DrawBuffers> {
    renderables
        .iter()
        .filter_map(|renderable| {
            let source = meshes.get(renderable.mesh)?;
            let material = materials.get(renderable.material)?;
            Some(bake_draw_buffers(
                device,
                source,
                material,
                &renderable.frame,
            ))
        })
        .collect()
}

/// Transforms `source`'s local-space positions by `frame` and tags every
/// vertex with `material.base_color_factor`, then uploads the result.
fn bake_draw_buffers(
    device: &Device,
    source: &MeshSource,
    material: &Material,
    frame: &Motor3,
) -> DrawBuffers {
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

    let index_bytes: Vec<u8> = source
        .indices
        .iter()
        .flat_map(|&i| (i as u16).to_le_bytes())
        .collect();
    let index_buffer = device.create_buffer(index_bytes.len(), BufferUsage::Index);
    device.write_buffer(&index_buffer, &index_bytes);

    DrawBuffers {
        vertex_buffer,
        index_buffer,
        index_count: source.indices.len() as u32,
    }
}

/// Owns the pipeline + camera uniform for rendering a [`Scene3D`] view.
/// One instance per view (a UI overlay's `Scene2D` is a separate,
/// future-work sibling — see docs/graphics-design.md); building it needs
/// a real GPU device and a surface to match color/depth formats against.
pub struct SceneRenderer {
    pipeline: RenderPipeline,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
}

impl SceneRenderer {
    pub fn new(device: &Device, surface: &Surface) -> Self {
        let shader = device.create_shader("meridian-unlit", UNLIT_SHADER_WGSL);
        let pipeline = device.create_render_pipeline(
            &shader,
            "vs_main",
            "fs_main",
            &unlit_vertex_layout(),
            surface,
            true,
        );
        let uniform_buffer = device.create_buffer(64, BufferUsage::Uniform);
        let bind_group = device.create_uniform_bind_group(&pipeline, &uniform_buffer);
        Self {
            pipeline,
            uniform_buffer,
            bind_group,
        }
    }

    /// Writes this view's view-projection matrix — call once per frame
    /// before [`draw`], not per draw call (see the module doc: this
    /// bridge has exactly one shared uniform per view).
    pub fn set_camera(&self, device: &Device, camera: &Camera) {
        device.write_buffer(
            &self.uniform_buffer,
            &mat4_to_bytes(camera.view_projection_matrix()),
        );
    }

    /// Records one draw call per `buffers` entry against `pass` — call
    /// inside the render pass, after [`prepare_draws`] built `buffers`
    /// and [`set_camera`](Self::set_camera) wrote this frame's matrix.
    pub fn draw(&self, pass: &mut DriverRenderPass<'_>, buffers: &[DrawBuffers]) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group);
        for entry in buffers {
            pass.set_vertex_buffer(0, &entry.vertex_buffer);
            pass.set_index_buffer_u16(&entry.index_buffer);
            pass.draw_indexed(0..entry.index_count);
        }
    }
}

/// The whole per-frame path for a `Scene3D` view in one call: bake
/// draw buffers for the (already-culled) `scene.renderables`, write the
/// camera uniform, and record the draws — everything [`prepare_draws`]
/// requires to happen before the pass, plus [`SceneRenderer::draw`]
/// inside it. Returns the buffers so the caller keeps them alive until
/// the surrounding `CommandBuffer::submit()` — see [`DrawBuffers`]'s doc
/// comment for why they can't just be dropped here.
pub fn submit_scene3d(
    device: &Device,
    renderer: &SceneRenderer,
    pass: &mut DriverRenderPass<'_>,
    scene: &Scene3D,
    meshes: &MeshRegistry,
    materials: &MaterialRegistry,
) -> Vec<DrawBuffers> {
    let buffers = prepare_draws(device, &scene.renderables, meshes, materials);
    renderer.set_camera(device, &scene.camera);
    renderer.draw(pass, &buffers);
    buffers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_registry_rejects_non_triangle_index_counts() {
        let mut registry = MeshRegistry::new();
        let source = MeshSource {
            positions: vec![[0.0, 0.0, 0.0]; 3],
            indices: vec![0, 1], // not a multiple of 3
        };
        assert_eq!(
            registry.register(source),
            Err(MeshRegistryError::NotATriangleList { index_count: 2 })
        );
    }

    #[test]
    fn mesh_registry_rejects_meshes_beyond_u16_vertex_range() {
        let mut registry = MeshRegistry::new();
        let source = MeshSource {
            positions: vec![[0.0, 0.0, 0.0]; u16::MAX as usize + 1],
            indices: vec![0, 1, 2],
        };
        assert_eq!(
            registry.register(source),
            Err(MeshRegistryError::TooManyVertices {
                count: u16::MAX as usize + 1
            })
        );
    }

    #[test]
    fn mesh_registry_round_trips_a_valid_mesh() {
        let mut registry = MeshRegistry::new();
        let source = MeshSource {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
        };
        let handle = registry.register(source).unwrap();
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
}
