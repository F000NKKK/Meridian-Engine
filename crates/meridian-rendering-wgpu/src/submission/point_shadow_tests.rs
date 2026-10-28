//! Tests of the point light's shadow cube in the lit uniform (ADR 041).

use meridian_gac::{Motor3, Vec3};
use meridian_rendering_domain::scene::{Light, Scene3d};
use meridian_rendering_domain::{
    Camera, POINT_SHADOW_NEAR, PointShadow, ShadowLight, ViewUniforms,
};

use super::environment::EnvironmentUniform;
use super::{lit_uniform_bytes, mat4_to_bytes};

/// Bytes of the point light's shadow cube at the end of the lit uniform.
const POINT_BYTES: usize = 6 * 64 + 48;

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
    Scene3d {
        lights: vec![Light::Point {
            position: Motor3::translation(Vec3::new(1.0, 2.0, 3.0)),
            color: [1.0; 3],
            intensity: 1.0,
            range: 12.0,
        }],
        ..Default::default()
    }
}

#[test]
fn a_point_shadow_ends_the_uniform_with_its_cube() {
    let scene = point_scene();
    let point = PointShadow::for_scene(&scene).unwrap();
    let shadow = ShadowLight {
        point: Some(point),
        ..ShadowLight::for_scene(&scene, 1024)
    };
    let bytes = uniform_for(&scene, &shadow);
    let block = &bytes[bytes.len() - POINT_BYTES..];
    let float = |at: usize| f32::from_le_bytes(block[at..at + 4].try_into().unwrap());
    assert_eq!(&block[..64], &mat4_to_bytes(point.view_proj[0])[..]);
    assert_eq!(
        [float(384), float(388), float(392), float(396)],
        [1.0, 2.0, 3.0, 12.0]
    );
    assert_eq!(float(400), POINT_SHADOW_NEAR);
    assert_eq!(u32_at(&bytes, bytes.len() - 16), 0);
}

#[test]
fn without_a_point_shadow_no_light_index_is_named() {
    let scene = Scene3d::default();
    let shadow = ShadowLight {
        point: None,
        ..ShadowLight::for_scene(&scene, 1024)
    };
    let bytes = uniform_for(&scene, &shadow);
    assert_eq!(u32_at(&bytes, bytes.len() - 16), u32::MAX);
}
