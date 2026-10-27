//! Tests of the submission adapter.

use meridian_gac_core::{Motor3, Vec3};
use meridian_graphics_core::scene::Light;
use meridian_graphics_core::{Camera, ShadowLight, ViewUniforms};

use super::{lit_uniform_bytes, mat4_to_bytes, *};

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

#[test]
fn lit_uniform_bytes_has_the_documented_layout_and_size() {
    let camera = Camera::default();
    let lights = vec![
        Light::Directional {
            direction: Motor3::identity(),
            color: [1.0, 1.0, 1.0],
            intensity: 2.0,
        },
        Light::Point {
            position: Motor3::translation(Vec3::new(1.0, 2.0, 3.0)),
            color: [0.5, 0.25, 0.0],
            intensity: 3.0,
            range: 10.0,
        },
    ];
    let identity = Camera::default().view_projection_matrix();
    let bytes = lit_uniform_bytes(&ViewUniforms {
        camera: &camera,
        shadow: ShadowLight {
            view_proj: identity,
            direction: Vec3::new(0.0, -1.0, 0.0),
            caster: 0,
        },
        ambient_ground: [0.1, 0.1, 0.1],
        ambient_sky: [0.2, 0.2, 0.2],
        lights: &lights,
    });
    assert_eq!(
        bytes.len(),
        64 + 64 + 16 + 16 + 16 + 16 + 16 + 16 + MAX_LIGHTS * 48
    );

    // light_count.x at offset 192 (view_proj[64] + light_view_proj[64]
    // + camera_pos[16] + ambient_ground[16] + ambient_sky[16] +
    // light_direction[16]) must read back as 2.
    let count = u32::from_le_bytes(bytes[192..196].try_into().unwrap());
    assert_eq!(count, 2);

    // shadow_caster.x at offset 208 (light_count's own 16 bytes
    // later) must read back as the `0` this test passed in.
    let shadow_caster = u32::from_le_bytes(bytes[208..212].try_into().unwrap());
    assert_eq!(shadow_caster, 0);

    // First light's kind field (lights array starts at 224, kind is
    // each 48-byte LightData's 4th f32).
    let lights_start = 64 + 64 + 16 + 16 + 16 + 16 + 16 + 16;
    let first_kind = f32::from_le_bytes(
        bytes[lights_start + 12..lights_start + 16]
            .try_into()
            .unwrap(),
    );
    assert_eq!(first_kind, 0.0); // directional
    let second_kind = f32::from_le_bytes(
        bytes[lights_start + 48 + 12..lights_start + 48 + 16]
            .try_into()
            .unwrap(),
    );
    assert_eq!(second_kind, 1.0); // point
}

#[test]
fn lit_uniform_bytes_caps_extra_lights_silently_at_the_byte_level() {
    // The truncation warning is logged by SceneRenderer::set_view;
    // lit_uniform_bytes itself just caps light_count and only writes
    // MAX_LIGHTS slots regardless of how many are passed in.
    let camera = Camera::default();
    let lights: Vec<Light> = (0..(MAX_LIGHTS + 3))
        .map(|_| Light::Directional {
            direction: Motor3::identity(),
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
        })
        .collect();
    let identity = Camera::default().view_projection_matrix();
    let bytes = lit_uniform_bytes(&ViewUniforms {
        camera: &camera,
        shadow: ShadowLight {
            view_proj: identity,
            direction: Vec3::new(0.0, -1.0, 0.0),
            caster: u32::MAX,
        },
        ambient_ground: [0.0, 0.0, 0.0],
        ambient_sky: [0.0, 0.0, 0.0],
        lights: &lights,
    });
    assert_eq!(
        bytes.len(),
        64 + 64 + 16 + 16 + 16 + 16 + 16 + 16 + MAX_LIGHTS * 48
    );
    let count = u32::from_le_bytes(bytes[192..196].try_into().unwrap());
    assert_eq!(count as usize, MAX_LIGHTS);
    let shadow_caster = u32::from_le_bytes(bytes[208..212].try_into().unwrap());
    assert_eq!(shadow_caster, u32::MAX);
}

/// `TextureRegistry::upload` against a real (or skipped) headless GPU
/// device — the one piece of this module testable without a window
/// (`SceneRenderer::new`/`prepare`/`draw` need a real `Surface`,
/// which needs a window; those are exercised end-to-end by a
/// windowed example instead, the same convention
/// `meridian-graphics-driver`'s own module doc states for its
/// windowed-only paths).
#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn texture_registry_round_trips_a_real_upload() {
    let Ok(device) = crate::Device::new().await else {
        eprintln!("skipping: no GPU device available");
        return;
    };
    let mut textures = TextureRegistry::new();
    let image = meridian_asset_core::ImageAsset {
        width: 1,
        height: 1,
        pixels: vec![255, 255, 255, 255],
    };
    let handle = textures.upload(&device, &image);
    assert!(textures.get(handle).is_some());
    assert!(textures.unregister(handle).is_some());
    assert!(textures.get(handle).is_none());
}
