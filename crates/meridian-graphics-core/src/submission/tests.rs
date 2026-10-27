//! Tests of this module.

use super::*;
use crate::Camera;
use crate::scene::{Light, Material};
use meridian_gac_core::{Motor3, Vec3};

fn triangle() -> MeshSource {
    MeshSource {
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        indices: vec![0, 1, 2],
    }
}

/// `m * vec4(p, 1)`, `m` column-major (`m[col][row]`) — the same
/// convention every matrix in this workspace uses (see
/// `Frustum::from_view_projection`'s identical row-extraction).
fn transform_point_via_matrix(m: [[f32; 4]; 4], p: Vec3) -> [f32; 4] {
    let v = [p.x, p.y, p.z, 1.0];
    let mut out = [0.0f32; 4];
    for (row, slot) in out.iter_mut().enumerate() {
        *slot = (0..4).map(|col| m[col][row] * v[col]).sum();
    }
    out
}

#[test]
fn directional_shadow_camera_orders_depth_by_distance_along_light_direction() {
    // Light shines straight down: a point above `scene_center` is
    // closer to the light than one below it.
    let light_direction = Vec3::new(0.0, -1.0, 0.0);
    let camera = directional_shadow_camera(light_direction, Vec3::ZERO);
    let vp = camera.view_projection_matrix();

    let near_clip = transform_point_via_matrix(vp, Vec3::new(0.0, 5.0, 0.0));
    let far_clip = transform_point_via_matrix(vp, Vec3::new(0.0, -5.0, 0.0));
    let near_depth = near_clip[2] / near_clip[3];
    let far_depth = far_clip[2] / far_clip[3];

    assert!(
        near_depth < far_depth,
        "a point closer to the light along its direction must have a smaller \
         depth (near {near_depth}, far {far_depth}) — matches the shadow \
         pipeline's CompareFunction::Less depth test"
    );
}

#[test]
fn directional_shadow_camera_centers_scene_center_in_clip_space() {
    let light_direction = Vec3::new(0.3, -1.0, 0.2);
    let scene_center = Vec3::new(1.0, 2.0, 3.0);
    let camera = directional_shadow_camera(light_direction, scene_center);
    let vp = camera.view_projection_matrix();

    let clip = transform_point_via_matrix(vp, scene_center);
    let ndc_x = clip[0] / clip[3];
    let ndc_y = clip[1] / clip[3];
    assert!(
        ndc_x.abs() < 1e-4,
        "scene_center must map to clip-space x=0, got {ndc_x}"
    );
    assert!(
        ndc_y.abs() < 1e-4,
        "scene_center must map to clip-space y=0, got {ndc_y}"
    );
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
        normals: vec![[0.0, 0.0, 1.0]; count],
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
fn mesh_registry_rejects_normal_count_mismatch() {
    let mut registry = MeshRegistry::new();
    let mut source = triangle();
    source.normals.pop();
    assert_eq!(
        registry.register(source),
        Err(MeshRegistryError::NormalCountMismatch {
            positions: 3,
            normals: 2
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
    let Ok(device) = meridian_graphics_driver::Device::new().await else {
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
