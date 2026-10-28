//! Tests of the sky backdrop.

use meridian_gac::{Motor3, Projection, Rotor, Vec3};
use meridian_rendering_domain::Camera;

use super::{sky_shader_wgsl, uniform_bytes};

fn floats(bytes: &[u8; 64]) -> [f32; 16] {
    std::array::from_fn(|i| f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap()))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn an_unrotated_camera_looks_along_x_with_z_to_its_right() {
    let camera = Camera {
        frame: Motor3::identity(),
        projection: Projection::perspective(1.0, 2.0, 0.1, 100.0),
    };
    let f = floats(&uniform_bytes(&camera, 3.0).unwrap());
    // right, up, forward
    assert!(close(f[2], 1.0) && close(f[5], 1.0) && close(f[8], 1.0));
    // half-extents of the view plane at distance 1, then the mip level
    let tan_y = (0.5f32).tan();
    assert!(close(f[13], tan_y) && close(f[12], tan_y * 2.0));
    assert!(close(f[14], 3.0));
}

#[test]
fn a_turned_camera_turns_its_axes() {
    let camera = Camera {
        frame: Motor3::from_rotation_translation(
            Rotor::from_axis_angle(Vec3::Y, core::f32::consts::FRAC_PI_2),
            Vec3::new(5.0, 1.0, 2.0),
        ),
        projection: Projection::perspective(1.0, 1.0, 0.1, 100.0),
    };
    let f = floats(&uniform_bytes(&camera, 0.0).unwrap());
    // Position does not matter; a quarter turn about +Y sends forward from +X to -Z.
    assert!(close(f[8], 0.0) && close(f[9], 0.0) && close(f[10], -1.0));
}

#[test]
fn a_camera_without_a_perspective_projection_has_no_sky() {
    let camera = Camera {
        frame: Motor3::identity(),
        projection: Projection::orthographic((-1.0, 1.0), (-1.0, 1.0), (0.1, 10.0)),
    };
    assert!(uniform_bytes(&camera, 0.0).is_none());
}

#[test]
fn the_shader_has_the_fullscreen_stage_and_a_fragment_entry() {
    let source = sky_shader_wgsl();
    assert!(source.contains("fn vs_main"));
    assert!(source.contains("fn fs_main"));
    assert!(source.contains("textureSampleLevel"));
}
