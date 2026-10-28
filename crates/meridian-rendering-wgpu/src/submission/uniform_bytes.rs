//! Packs matrices, vectors and lights into uniform buffer bytes.

use meridian_gac::Vec3;
use meridian_rendering_domain::scene::Light;
use meridian_rendering_domain::{SHADOW_CASCADES, ShadowCascades, ShadowLight, ViewUniforms};

use super::MAX_LIGHTS;
use super::environment::EnvironmentUniform;

/// Bytes of the lit uniform: the view, the cascades, the lights, then the environment.
pub(crate) const LIT_UNIFORM_BYTES: usize = 640 + EnvironmentUniform::BYTES;

// The WGSL `Uniforms` holds the cascades as an `array<mat4x4, 4>` and two `vec4`s.
const _: () = assert!(SHADOW_CASCADES == 4);

pub(crate) fn mat4_to_bytes(m: [[f32; 4]; 4]) -> [u8; 64] {
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

fn vec3_to_padded_bytes(v: [f32; 3]) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    for (i, component) in v.iter().enumerate() {
        bytes[i * 4..i * 4 + 4].copy_from_slice(&component.to_le_bytes());
    }
    bytes
}

/// Serializes a view's camera, shadow light, ambient colors and lights into the lit shaders'
/// shared `Uniforms` layout (see [`LIT_UNIFORMS_AND_SHADING_WGSL`](super::LIT_UNIFORMS_AND_SHADING_WGSL)):
/// `view_proj` (64 bytes), the four cascades' `light_view_proj` (4 x 64),
/// `cascade_far` and `cascade_bias` (16 each), `camera_pos` (16),
/// `ambient_ground` (16), `ambient_sky` (16), `light_direction` (16),
/// `light_count` (16), `shadow_caster` (16), then [`MAX_LIGHTS`](super::MAX_LIGHTS)
/// `LightData` entries (48 bytes each), then the environment's `env_sh` (9 x 16)
/// and `env_params` (16) — 800 bytes total. Lights beyond
/// the cap are dropped (logged once by the caller, not here — see
/// `bake_batches`). `shadow_caster` is the index into
/// `lights` of the shadow-casting light (see
/// [`SceneRenderer::render_shadow_pass`](super::SceneRenderer::render_shadow_pass)), or `u32::MAX` if none —
/// `shade`'s shadow lookup only ever matches a real light index, never
/// `u32::MAX`, so "no shadow-casting light" safely degrades to "no
/// light gets a shadow factor applied," not an out-of-bounds index.
/// `light_direction` is that same light's travel direction (unit
/// vector), independent of `shadow_caster` — the shadow lookup uses it
/// directly for its slope-scaled bias rather than re-indexing `lights`.
pub(crate) fn lit_uniform_bytes(
    view: &ViewUniforms<'_>,
    environment: EnvironmentUniform,
) -> Vec<u8> {
    let ShadowLight {
        cascades,
        direction,
        caster,
    } = view.shadow;
    let camera_pos = view.camera.frame.transform_point(Vec3::ZERO);
    let count = view.lights.len().min(MAX_LIGHTS) as u32;
    let mut bytes = Vec::with_capacity(LIT_UNIFORM_BYTES);
    bytes.extend_from_slice(&mat4_to_bytes(view.camera.view_projection_matrix()));
    bytes.extend_from_slice(&cascades_to_bytes(&cascades));
    bytes.extend_from_slice(&vec3_to_padded_bytes([
        camera_pos.x,
        camera_pos.y,
        camera_pos.z,
    ]));
    bytes.extend_from_slice(&vec3_to_padded_bytes(view.ambient_ground));
    bytes.extend_from_slice(&vec3_to_padded_bytes(view.ambient_sky));
    bytes.extend_from_slice(&vec3_to_padded_bytes([
        direction.x,
        direction.y,
        direction.z,
    ]));
    bytes.extend_from_slice(&u32_to_padded_bytes(count));
    bytes.extend_from_slice(&u32_to_padded_bytes(caster));
    for index in 0..MAX_LIGHTS {
        bytes.extend_from_slice(&light_to_bytes(view.lights.get(index)));
    }
    bytes.extend_from_slice(&environment.to_bytes());
    bytes
}

/// The cascades' matrices, then their far edges and depth biases as two `vec4`s.
fn cascades_to_bytes(cascades: &ShadowCascades) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(SHADOW_CASCADES * 64 + 32);
    for view_proj in cascades.view_proj {
        bytes.extend_from_slice(&mat4_to_bytes(view_proj));
    }
    for value in cascades
        .far_edges
        .into_iter()
        .chain(cascades.bias_per_texel)
    {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// A `u32` padded to a `vec4<u32>`.
fn u32_to_padded_bytes(value: u32) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[..4].copy_from_slice(&value.to_le_bytes());
    bytes
}

/// One 48-byte `LightData` entry: position or direction, kind
/// (0 directional, 1 point), color, intensity, range; all zero for an
/// unused slot.
fn light_to_bytes(light: Option<&Light>) -> [u8; 48] {
    let fields: [f32; 12] = match light {
        Some(Light::Directional {
            direction,
            color,
            intensity,
        }) => {
            let dir: Vec3 = direction.transform_vector(Vec3::X);
            [
                dir.x, dir.y, dir.z, 0.0, color[0], color[1], color[2], *intensity, 0.0, 0.0, 0.0,
                0.0,
            ]
        }
        Some(Light::Point {
            position,
            color,
            intensity,
            range,
        }) => {
            let pos: Vec3 = position.transform_point(Vec3::ZERO);
            [
                pos.x, pos.y, pos.z, 1.0, color[0], color[1], color[2], *intensity, *range, 0.0,
                0.0, 0.0,
            ]
        }
        None => [0.0; 12],
    };
    let mut bytes = [0u8; 48];
    for (index, field) in fields.iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&field.to_le_bytes());
    }
    bytes
}
