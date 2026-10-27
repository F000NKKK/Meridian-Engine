//! GPU buffers baked for one frame's draws.

use meridian_gac::Vec3;

use super::{DrawKind, DrawRequest, SceneRenderer};
use crate::{Buffer, BufferUsage, Device, SampledTexture, Texture};

/// One renderable's baked-for-this-frame GPU state — built by
/// [`SceneRenderer::prepare`](super::SceneRenderer::prepare) *before* the render pass opens, kept alive
/// by the caller until [`SceneRenderer::draw`](super::SceneRenderer::draw) (and the pass itself) is
/// done with them. This split exists because `RenderPass::set_vertex_buffer`
/// borrows its buffer for the pass's lifetime — the same reason
/// `soft_body_rubber_balls`'s own `ball_buffers` are built up front (see
/// that example's module doc). `crate::bloom` redraws the same
/// `vertex_buffer`/`index_buffer` through its own emissive-extraction
/// pipeline, so both stay `pub(crate)`.
#[derive(Debug)]
pub struct DrawBuffers {
    pub(crate) vertex_buffer: Buffer,
    pub(crate) index_buffer: Buffer,
    pub(crate) index_count: u32,
    pub(crate) lit: bool,
    pub(crate) casts_shadow: bool,
    pub(crate) kind: DrawKind,
}

/// Transforms the request's local-space positions/normals by its frame,
/// bakes in the material's tint/emissive (and UVs, for the textured
/// path), and uploads the result — the flat/unlit-vertex-layout path when
/// there is no albedo, the textured one (with a bind group naming the
/// albedo) otherwise. `lit` only affects which pipeline [`SceneRenderer::draw`](super::SceneRenderer::draw)
/// selects; the baked bytes are identical either way (see the module
/// doc: both lit and unlit pipelines within a family share one vertex
/// layout).
pub(crate) fn bake_draw_buffers(
    device: &Device,
    renderer: &SceneRenderer,
    request: &DrawRequest<'_>,
) -> DrawBuffers {
    let source = request.mesh;
    let index_bytes: Vec<u8> = source
        .indices
        .iter()
        .flat_map(|&i| (i as u16).to_le_bytes())
        .collect();
    let index_buffer = device.create_buffer(index_bytes.len(), BufferUsage::Index);
    device.write_buffer(&index_buffer, &index_bytes);
    let lit = !request.material.unlit;

    let (vertex_bytes, kind) = match request.albedo {
        Some(texture) => (
            textured_vertex_bytes(request),
            DrawKind::Textured {
                bind_group: textured_bind_group(device, renderer, texture, lit),
            },
        ),
        None => (colored_vertex_bytes(request), DrawKind::Colored),
    };
    let vertex_buffer = device.create_buffer(vertex_bytes.len(), BufferUsage::Vertex);
    device.write_buffer(&vertex_buffer, &vertex_bytes);
    DrawBuffers {
        vertex_buffer,
        index_buffer,
        index_count: source.indices.len() as u32,
        lit,
        casts_shadow: request.casts_shadow,
        kind,
    }
}

/// Appends `values` as little-endian `f32`s.
fn push_f32s(bytes: &mut Vec<u8>, values: &[f32]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

/// Appends a vertex's world-space position and normal.
fn push_world_vertex(bytes: &mut Vec<u8>, request: &DrawRequest<'_>, index: usize) {
    let [x, y, z] = request.mesh.positions[index];
    let [nx, ny, nz] = request.mesh.normals[index];
    let world = request.frame.transform_point(Vec3::new(x, y, z));
    let normal = request.frame.transform_vector(Vec3::new(nx, ny, nz));
    push_f32s(
        bytes,
        &[world.x, world.y, world.z, normal.x, normal.y, normal.z],
    );
}

/// Position, normal, tint and emissive per vertex (52 bytes).
fn colored_vertex_bytes(request: &DrawRequest<'_>) -> Vec<u8> {
    let count = request.mesh.positions.len().min(request.mesh.normals.len());
    let mut bytes = Vec::with_capacity(count * 52);
    for index in 0..count {
        push_world_vertex(&mut bytes, request, index);
        push_f32s(&mut bytes, &request.material.base_color_factor);
        push_f32s(&mut bytes, &request.material.emissive);
    }
    bytes
}

/// Position, normal, UV, tint and emissive per vertex (60 bytes).
fn textured_vertex_bytes(request: &DrawRequest<'_>) -> Vec<u8> {
    let mesh = request.mesh;
    let count = mesh
        .positions
        .len()
        .min(mesh.normals.len())
        .min(mesh.uvs.len());
    let mut bytes = Vec::with_capacity(count * 60);
    for index in 0..count {
        push_world_vertex(&mut bytes, request, index);
        push_f32s(&mut bytes, &mesh.uvs[index]);
        push_f32s(&mut bytes, &request.material.base_color_factor);
        push_f32s(&mut bytes, &request.material.emissive);
    }
    bytes
}

/// A fresh bind group naming `texture`. Lit textured draws need the shadow
/// map/sampler in their bind group too (see `lit_textured_shader_wgsl`'s
/// bindings 3/4); unlit ignores lighting entirely, so its bind group stays
/// the plain 3-binding shape.
fn textured_bind_group(
    device: &Device,
    renderer: &SceneRenderer,
    texture: &Texture,
    lit: bool,
) -> crate::BindGroup {
    if lit {
        device.create_textured_shadow_bind_group(
            &renderer.textured_lit_pipeline,
            &renderer.uniform_buffer,
            SampledTexture {
                texture,
                sampler: &renderer.sampler,
            },
            SampledTexture {
                texture: &renderer.shadow_map,
                sampler: &renderer.shadow_sampler,
            },
        )
    } else {
        device.create_textured_bind_group(
            &renderer.textured_unlit_pipeline,
            &renderer.uniform_buffer,
            texture,
            &renderer.sampler,
        )
    }
}
