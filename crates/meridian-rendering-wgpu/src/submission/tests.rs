//! Tests of the submission adapter.

use meridian_gac::{Motor3, Vec3};
use meridian_rendering_domain::scene::{Light, Material};
use meridian_rendering_domain::{
    Camera, MAX_POINT_SHADOWS, ShadowCascades, ShadowLight, ViewUniforms,
};

use super::environment::EnvironmentUniform;
use super::{lit_uniform_bytes, mat4_to_bytes, *};

/// Where the lights array starts: `view_proj`, four cascades with their far
/// edges and biases, then `camera_pos`, both ambients, `light_direction`,
/// `light_count` and `shadow_caster`.
const LIGHTS_START: usize = 64 + 4 * 64 + 16 + 16 + 6 * 16;

/// The `u32` at `offset`.
/// Bytes of the point light's shadow cube at the end of the lit uniform.
const POINT_BYTES: usize = 4 * (6 * 64 + 16 + 4) + 16;

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
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
    let bytes = lit_uniform_bytes(
        &ViewUniforms {
            camera: &camera,
            shadow: ShadowLight {
                cascades: ShadowCascades::fixed(Vec3::new(0.0, -1.0, 0.0), 1024),
                direction: Vec3::new(0.0, -1.0, 0.0),
                caster: 0,
                points: [None; MAX_POINT_SHADOWS],
            },
            ambient_ground: [0.1, 0.1, 0.1],
            ambient_sky: [0.2, 0.2, 0.2],
            lights: &lights,
        },
        EnvironmentUniform::none(),
    );
    assert_eq!(
        bytes.len(),
        LIGHTS_START + MAX_LIGHTS * 48 + EnvironmentUniform::BYTES + POINT_BYTES
    );

    let count = u32_at(&bytes, LIGHTS_START - 32);
    assert_eq!(count, 2);

    let shadow_caster = u32_at(&bytes, LIGHTS_START - 16);
    assert_eq!(shadow_caster, 0);

    // First light's kind field (kind is
    // each 48-byte LightData's 4th f32).
    let lights_start = LIGHTS_START;
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
    let bytes = lit_uniform_bytes(
        &ViewUniforms {
            camera: &camera,
            shadow: ShadowLight {
                cascades: ShadowCascades::fixed(Vec3::new(0.0, -1.0, 0.0), 1024),
                direction: Vec3::new(0.0, -1.0, 0.0),
                caster: u32::MAX,
                points: [None; MAX_POINT_SHADOWS],
            },
            ambient_ground: [0.0, 0.0, 0.0],
            ambient_sky: [0.0, 0.0, 0.0],
            lights: &lights,
        },
        EnvironmentUniform::none(),
    );
    assert_eq!(
        bytes.len(),
        LIGHTS_START + MAX_LIGHTS * 48 + EnvironmentUniform::BYTES + POINT_BYTES
    );
    let count = u32::from_le_bytes(
        bytes[LIGHTS_START - 32..LIGHTS_START - 28]
            .try_into()
            .unwrap(),
    );
    assert_eq!(count as usize, MAX_LIGHTS);
    let shadow_caster = u32::from_le_bytes(
        bytes[LIGHTS_START - 16..LIGHTS_START - 12]
            .try_into()
            .unwrap(),
    );
    assert_eq!(shadow_caster, u32::MAX);
}

/// `TextureRegistry::upload` against a real (or skipped) headless GPU
/// device — the one piece of this module testable without a window
/// (`SceneRenderer::new`/`prepare`/`draw` need a real `Surface`,
/// which needs a window; those are exercised end-to-end by a
/// windowed example instead, the same convention
/// `meridian-rendering-wgpu`'s own module doc states for its
/// windowed-only paths).
#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn texture_registry_round_trips_a_real_upload() {
    let Ok(device) = crate::Device::new().await else {
        eprintln!("skipping: no GPU device available");
        return;
    };
    let mut textures = TextureRegistry::new();
    let image = meridian_asset_domain::ImageAsset {
        width: 1,
        height: 1,
        pixels: vec![255, 255, 255, 255],
    };
    let handle = textures.upload(&device, &image);
    assert!(textures.get(handle).is_some());
    assert!(textures.unregister(handle).is_some());
    assert!(textures.get(handle).is_none());
}

#[test]
fn an_instance_record_fills_its_layout_and_carries_matrix_and_surface() {
    use super::instance_data::push_instance;

    let material = Material {
        base_color_factor: [0.1, 0.2, 0.3, 0.4],
        emissive: [0.5, 0.6, 0.7],
        roughness: 0.25,
        metallic: 0.75,
        normal_scale: 0.5,
        occlusion_strength: 0.125,
        ..Material::default()
    };
    let frame = Motor3::translation(Vec3::new(1.0, 2.0, 3.0));
    let mut bytes = Vec::new();
    push_instance(&mut bytes, &frame, &material);

    assert_eq!(
        u64::try_from(bytes.len()).unwrap(),
        instance_vertex_layout().stride
    );
    let float =
        |index: usize| f32::from_le_bytes(bytes[index * 4..index * 4 + 4].try_into().unwrap());
    // The fourth matrix column is the translation, column-major.
    assert_eq!(
        [float(12), float(13), float(14), float(15)],
        [1.0, 2.0, 3.0, 1.0]
    );
    assert_eq!(
        [float(16), float(17), float(18), float(19)],
        [0.1, 0.2, 0.3, 0.4]
    );
    assert_eq!([float(20), float(21), float(22)], [0.5, 0.6, 0.7]);
    assert_eq!([float(23), float(24)], [0.25, 0.75]);
    assert_eq!([float(25), float(26)], [0.5, 0.125]);
    // No emissive map: the factor stays in the plain slot, the mapped one is zero.
    assert_eq!([float(27), float(28), float(29)], [0.0; 3]);
}

#[test]
fn a_mapped_emissive_goes_to_its_own_slot_and_bloom_reads_none() {
    use super::instance_data::push_instance;

    let material = Material {
        emissive: [0.5, 0.6, 0.7],
        emissive_map: Some(meridian_rendering_domain::TextureHandle::new(
            meridian_memory::Handle {
                index: 0,
                generation: 0,
            },
        )),
        ..Material::default()
    };
    let mut bytes = Vec::new();
    push_instance(&mut bytes, &Motor3::identity(), &material);
    let float =
        |index: usize| f32::from_le_bytes(bytes[index * 4..index * 4 + 4].try_into().unwrap());
    assert_eq!(
        [float(20), float(21), float(22)],
        [0.0; 3],
        "bloom reads none"
    );
    assert_eq!([float(27), float(28), float(29)], [0.5, 0.6, 0.7]);
}

#[test]
fn every_attribute_lies_within_its_layouts_stride() {
    for layout in [
        mesh_vertex_layout(),
        instance_vertex_layout(),
        position_only_mesh_layout(),
        shadow_instance_layout(),
        emissive_instance_layout(),
        emissive_mapped_instance_layout(),
    ] {
        for attribute in &layout.attributes {
            assert!(
                attribute.offset + attribute.format.size() <= layout.stride,
                "location {} overruns the stride",
                attribute.location
            );
        }
    }
}

#[test]
fn the_default_material_is_a_fairly_rough_dielectric() {
    let material = Material::default();
    assert_eq!(material.roughness, 0.5);
    assert_eq!(material.metallic, 0.0);
}

#[test]
fn an_environment_uniform_writes_its_coefficients_then_its_flags() {
    let map =
        meridian_rendering_domain::EnvironmentMap::from_equirect(8, 4, &[[0.5; 3]; 32]).unwrap();
    let bytes = EnvironmentUniform::of(&map).to_bytes();
    let float = |offset: usize| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    // Coefficient 0 of a uniform sky is its colour over 0.282095 (the band-0 basis value).
    assert!((float(0) * 0.282_095 - 0.5).abs() < 0.02);
    assert_eq!(float(9 * 16), 1.0, "enabled");
    assert_eq!(float(9 * 16 + 4), 5.0, "six levels, highest mip 5");
    assert_eq!(
        EnvironmentUniform::none().to_bytes()[9 * 16..9 * 16 + 4],
        0.0f32.to_le_bytes()
    );
}
