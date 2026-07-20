//! Shared rendering scaffolding for the soft-body examples
//! (`soft_body_rubber_balls`, `soft_body_jiggle_deterministic`,
//! `soft_body_jiggle_float`) — the mesh-topology/vertex-buffer plumbing
//! all three need is identical; only the physics backend and scenario
//! differ between them. Not part of any published crate (`publish =
//! false` on this package) — this is example-only scaffolding, not a
//! new engine API.
//!
//! Unlike `spinning_cube`'s shader (which takes local-space vertices and
//! a `model` matrix), soft-body particle positions are already
//! world-space (`icosphere_soft_body`'s `center` offset is baked into
//! every particle at construction), so [`SOFT_BODY_SHADER`]'s uniform is
//! just a view-projection matrix, no per-mesh model matrix.

use meridian_gac_core::Vec3;
use meridian_gac_core::generic::Face;
use meridian_graphics_driver::{VertexAttributeDesc, VertexFormat, VertexLayout};

pub const SOFT_BODY_SHADER: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.world_normal = in.normal;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 0.8, 0.3));
    let diffuse = max(dot(normalize(in.world_normal), light_dir), 0.0);
    let ambient = 0.2;
    let base_color = vec3<f32>(0.85, 0.3, 0.25);
    let color = base_color * (ambient + diffuse * 0.8);
    return vec4<f32>(color, 1.0);
}
"#;

pub fn soft_body_vertex_layout() -> VertexLayout {
    VertexLayout {
        stride: 24, // position (12 bytes) + normal (12 bytes)
        attributes: vec![
            VertexAttributeDesc {
                location: 0,
                format: VertexFormat::Float32x3,
                offset: 0,
            },
            VertexAttributeDesc {
                location: 1,
                format: VertexFormat::Float32x3,
                offset: 12,
            },
        ],
    }
}

/// Builds `(vertex_bytes, index_bytes, index_count)` for a soft body's
/// current surface shape: `surface_positions` (world-space, the
/// `SoftBody`'s particles excluding the interior center particle — see
/// `icosphere_soft_body`'s own doc comment) plus `faces` (the
/// icosphere's fixed triangle topology — same `subdivisions` always
/// produces the same face list, independent of how the vertices have
/// since deformed, so callers build it once via
/// [`meridian_gac_core::icosphere`] and reuse it every frame).
///
/// Flat per-face normals are recomputed from `surface_positions` every
/// call (not cached) so lighting responds to deformation — each face
/// gets its own un-shared vertices (like `spinning_cube`'s cube mesh),
/// trading vertex count for genuinely flat shading instead of
/// interpolated/averaged normals that would blur a sharply dented face.
pub fn soft_body_render_buffers(
    surface_positions: &[Vec3],
    faces: &[Face],
) -> (Vec<u8>, Vec<u8>, u32) {
    let mut vertex_bytes = Vec::new();
    let mut index_bytes = Vec::new();
    let mut index_count = 0u32;
    let mut next_index: u16 = 0;

    for face in faces {
        for (a, b, c) in face.triangles() {
            let pa = surface_positions[a];
            let pb = surface_positions[b];
            let pc = surface_positions[c];
            let normal = (pb - pa).cross(pc - pa).normalize();

            for p in [pa, pb, pc] {
                for component in [p.x, p.y, p.z, normal.x, normal.y, normal.z] {
                    vertex_bytes.extend_from_slice(&component.to_le_bytes());
                }
                index_bytes.extend_from_slice(&next_index.to_le_bytes());
                next_index += 1;
                index_count += 1;
            }
        }
    }

    (vertex_bytes, index_bytes, index_count)
}

pub fn mat4_to_bytes(m: [[f32; 4]; 4]) -> [u8; 64] {
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

/// A camera rotor turning `gac-core`'s local-forward `+X` toward `target
/// - eye` — see `spinning_cube`'s identical helper, duplicated here
/// (not worth a shared engine-level utility for three examples) rather
/// than depended on across two `[[example]]` binaries.
pub fn look_at_rotor(eye: Vec3, target: Vec3) -> meridian_gac_core::Rotor {
    use meridian_gac_core::Rotor;
    let forward = (target - eye).normalize();
    let local_forward = Vec3::X;
    let cos_angle = local_forward.dot(forward).clamp(-1.0, 1.0);
    if cos_angle > 0.9999 {
        return Rotor::identity();
    }
    if cos_angle < -0.9999 {
        return Rotor::from_axis_angle(Vec3::Y, core::f32::consts::PI);
    }
    let axis = local_forward.cross(forward).normalize();
    Rotor::from_axis_angle(axis, cos_angle.acos())
}
