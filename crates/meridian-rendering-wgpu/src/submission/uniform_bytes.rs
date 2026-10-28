//! Packs matrices, vectors and lights into uniform buffer bytes.

use meridian_gac::Vec3;
use meridian_rendering_domain::scene::Light;
use meridian_rendering_domain::{
    LocalShadow, MAX_LOCAL_SHADOWS, POINT_SHADOW_FACES, POINT_SHADOW_NEAR, SHADOW_CASCADES,
    ShadowCascades, ShadowLight, ViewUniforms, spot_cone,
};

use super::MAX_LIGHTS;
use super::environment::EnvironmentUniform;

/// Bytes of one `LightData` entry.
const LIGHT_BYTES: usize = 64;

/// Bytes of the lit uniform: the view, the cascades, the lights, then the environment.
pub(crate) const LIT_UNIFORM_BYTES: usize =
    448 + MAX_LIGHTS * LIGHT_BYTES + EnvironmentUniform::BYTES + LOCAL_SHADOW_UNIFORM_BYTES;

/// The local lights' shadows: six matrices per slot (a spot light fills the
/// first), then each slot's position and range, the near plane, and each
/// slot's light index.
const LOCAL_SHADOW_UNIFORM_BYTES: usize =
    MAX_LOCAL_SHADOWS * (POINT_SHADOW_FACES * 64 + 16 + 4) + 16;

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
/// `LightData` entries (64 bytes each), then the environment's `env_sh` (9 x 16)
/// and `env_params` (16), then the local lights' shadows (see
/// `local_shadows_to_bytes`) — 1632 bytes more. Lights beyond
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
        locals,
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
    bytes.extend_from_slice(&local_shadows_to_bytes(&locals));
    bytes
}

/// The local lights' shadows as the shader's `point_view_proj` (six
/// matrices per slot, the first one for a spot light), `point_position`
/// (position and range per slot), `point_params` (the near plane) and
/// `point_casters` (each slot's light index, `u32::MAX` for an empty slot,
/// which no light matches).
fn local_shadows_to_bytes(locals: &[Option<LocalShadow>; MAX_LOCAL_SHADOWS]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(LOCAL_SHADOW_UNIFORM_BYTES);
    for local in locals {
        let mut faces = [[[0.0; 4]; 4]; POINT_SHADOW_FACES];
        match local {
            Some(LocalShadow::Point(point)) => faces = point.view_proj,
            Some(LocalShadow::Spot(spot)) => faces[0] = spot.view_proj,
            None => {}
        }
        for view_proj in faces {
            bytes.extend_from_slice(&mat4_to_bytes(view_proj));
        }
    }
    for local in locals {
        let (position, far) = local
            .as_ref()
            .map_or((Vec3::ZERO, 0.0), |l| (l.position(), l.far()));
        for value in [position.x, position.y, position.z, far] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for value in [POINT_SHADOW_NEAR, 0.0, 0.0, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for local in locals {
        let caster = local.as_ref().map_or(u32::MAX, LocalShadow::caster);
        bytes.extend_from_slice(&caster.to_le_bytes());
    }
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

/// One 64-byte `LightData` entry: position or direction, kind
/// (0 directional, 1 point, 2 spot), color, intensity, range and the cone's
/// falloff scale, then a spot light's direction and the cosine of its outer
/// angle; all zero for an unused slot.
fn light_to_bytes(light: Option<&Light>) -> [u8; LIGHT_BYTES] {
    let fields: [f32; 16] = match light {
        Some(Light::Directional {
            direction,
            color,
            intensity,
        }) => {
            let dir: Vec3 = direction.transform_vector(Vec3::X);
            [
                dir.x, dir.y, dir.z, 0.0, color[0], color[1], color[2], *intensity, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0,
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
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ]
        }
        Some(Light::Spot {
            position,
            direction,
            color,
            intensity,
            range,
            inner_angle,
            outer_angle,
        }) => {
            let pos: Vec3 = position.transform_point(Vec3::ZERO);
            let dir: Vec3 = direction.transform_vector(Vec3::X);
            let (cos_inner, cos_outer) = spot_cone(*inner_angle, *outer_angle);
            let scale = 1.0 / (cos_inner - cos_outer).max(1e-4);
            [
                pos.x, pos.y, pos.z, 2.0, color[0], color[1], color[2], *intensity, *range, scale,
                0.0, 0.0, dir.x, dir.y, dir.z, cos_outer,
            ]
        }
        None => [0.0; 16],
    };
    let mut bytes = [0u8; LIGHT_BYTES];
    for (index, field) in fields.iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&field.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod point_shadow_tests {
    use meridian_gac::{Motor3, Vec3};
    use meridian_rendering_domain::scene::{Light, Scene3d};
    use meridian_rendering_domain::{Camera, POINT_SHADOW_NEAR, ShadowLight, ViewUniforms};

    use super::*;

    /// Bytes of the point light's shadow cube at the end of the lit uniform.
    const POINT_BYTES: usize = LOCAL_SHADOW_UNIFORM_BYTES;

    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    fn uniform_for(scene: &Scene3d, shadow: &ShadowLight) -> Vec<u8> {
        lit_uniform_bytes(
            &ViewUniforms {
                camera: &Camera::default(),
                shadow: *shadow,
                ambient_ground: [0.0; 3],
                ambient_sky: [0.0; 3],
                lights: &scene.lights,
            },
            EnvironmentUniform::none(),
        )
    }

    fn point_scene() -> Scene3d {
        let point = |x: f32, range: f32| Light::Point {
            position: Motor3::translation(Vec3::new(x, 2.0, 3.0)),
            color: [1.0; 3],
            intensity: 1.0,
            range,
        };
        Scene3d {
            lights: vec![point(9.0, 8.0), point(1.0, 12.0)],
            ..Default::default()
        }
    }

    #[test]
    fn point_shadows_end_the_uniform_slot_by_slot_nearest_first() {
        let scene = point_scene();
        let shadow = ShadowLight::for_scene(&scene, 1024);
        let Some(LocalShadow::Point(near)) = shadow.locals[0] else {
            panic!("the nearest light is a point light");
        };
        let bytes = uniform_for(&scene, &shadow);
        let block = &bytes[bytes.len() - POINT_BYTES..];
        let float = |at: usize| f32::from_le_bytes(block[at..at + 4].try_into().unwrap());
        assert_eq!(&block[..64], &mat4_to_bytes(near.view_proj[0])[..]);
        assert_eq!(
            [float(1536), float(1540), float(1544), float(1548)],
            [1.0, 2.0, 3.0, 12.0],
            "slot 0 is the light nearest the camera"
        );
        assert_eq!(float(1552), 9.0, "slot 1 follows");
        assert_eq!(float(1600), POINT_SHADOW_NEAR);
        let casters: Vec<u32> = (0..4)
            .map(|slot| u32_at(&bytes, bytes.len() - 16 + slot * 4))
            .collect();
        assert_eq!(casters, [1, 0, u32::MAX, u32::MAX]);
    }

    #[test]
    fn a_spot_light_fills_the_first_matrix_of_its_slot_and_its_light_entry() {
        let scene = Scene3d {
            lights: vec![Light::Spot {
                position: Motor3::translation(Vec3::new(1.0, 2.0, 3.0)),
                direction: Motor3::identity(),
                color: [1.0, 0.5, 0.25],
                intensity: 2.0,
                range: 12.0,
                inner_angle: 0.3,
                outer_angle: 0.6,
            }],
            ..Default::default()
        };
        let shadow = ShadowLight::for_scene(&scene, 1024);
        let Some(LocalShadow::Spot(spot)) = shadow.locals[0] else {
            panic!("the light is a spot light");
        };
        let bytes = uniform_for(&scene, &shadow);
        let block = &bytes[bytes.len() - POINT_BYTES..];
        assert_eq!(&block[..64], &mat4_to_bytes(spot.view_proj)[..]);
        assert!(
            block[64..384].iter().all(|b| *b == 0),
            "the other faces are empty"
        );
        let float = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let light = 448;
        assert_eq!(float(light + 12), 2.0, "kind 2 is a spot light");
        assert_eq!(float(light + 48), 1.0, "the cone points along +X");
        assert!((float(light + 60) - 0.6f32.cos()).abs() < 1e-6);
        let scale = 1.0 / (0.3f32.cos() - 0.6f32.cos());
        assert!((float(light + 36) - scale).abs() < 1e-4);
    }

    #[test]
    fn without_point_shadows_no_light_index_is_named() {
        let scene = Scene3d::default();
        let shadow = ShadowLight::for_scene(&scene, 1024);
        let bytes = uniform_for(&scene, &shadow);
        let casters: Vec<u32> = (0..4)
            .map(|slot| u32_at(&bytes, bytes.len() - 16 + slot * 4))
            .collect();
        assert_eq!(casters, [u32::MAX; 4]);
    }
}
