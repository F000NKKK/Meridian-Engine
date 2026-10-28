//! Tests of the shadow cascades.

use meridian_gac::{Motor3, Projection, Rotor, Vec3};

use super::*;
use crate::Camera;

const TILE: u32 = 1024;
const TILE_TEXELS: f32 = 1024.0;

fn camera_at(position: Vec3, yaw: f32) -> Camera {
    Camera {
        frame: Motor3::from_rotation_translation(Rotor::from_axis_angle(Vec3::Y, yaw), position),
        projection: Projection::perspective(1.0, 16.0 / 9.0, 0.1, 500.0),
    }
}

fn sun() -> Vec3 {
    Vec3::new(0.3, -1.0, 0.2).normalize()
}

/// `m * (p, 1)` with the perspective divide, `m` column-major.
fn project(m: [[f32; 4]; 4], p: Vec3) -> [f32; 3] {
    let v = [p.x, p.y, p.z, 1.0];
    let row = |r: usize| (0..4).map(|c| m[c][r] * v[c]).sum::<f32>();
    let w = row(3);
    [row(0) / w, row(1) / w, row(2) / w]
}

/// World point at view distance `depth`, `x` and `y` fractions of the half-extent.
fn point_in_view(camera: &Camera, depth: f32, x: f32, y: f32) -> Vec3 {
    let tan_y = (0.5f32).tan();
    let tan_x = tan_y * 16.0 / 9.0;
    camera
        .frame
        .transform_point(Vec3::new(depth, y * depth * tan_y, x * depth * tan_x))
}

#[test]
fn splits_grow_with_distance_and_end_at_the_shadow_distance() {
    let cascades = ShadowCascades::fit(&camera_at(Vec3::ZERO, 0.0), sun(), TILE);
    let edges = cascades.far_edges;
    assert!(edges.windows(2).all(|pair| pair[0] < pair[1]), "{edges:?}");
    assert!(edges[0] > 0.1);
    assert!((edges[SHADOW_CASCADES - 1] - MAX_SHADOW_DISTANCE).abs() < 1e-3);
}

#[test]
fn a_nearer_far_plane_ends_the_last_cascade_there() {
    let mut camera = camera_at(Vec3::ZERO, 0.0);
    camera.projection = Projection::perspective(1.0, 16.0 / 9.0, 0.1, 40.0);
    let cascades = ShadowCascades::fit(&camera, sun(), TILE);
    assert!((cascades.far_edges[SHADOW_CASCADES - 1] - 40.0).abs() < 1e-2);
}

#[test]
fn every_cascade_contains_its_slice_of_the_view() {
    let camera = camera_at(Vec3::new(3.0, 2.0, -5.0), 0.7);
    let cascades = ShadowCascades::fit(&camera, sun(), TILE);
    let mut start = 0.1;
    for (index, &end) in cascades.far_edges.iter().enumerate() {
        for depth in [start + 0.01, (start + end) * 0.5, end - 0.01] {
            for (x, y) in [
                (-1.0, -1.0),
                (1.0, 1.0),
                (-1.0, 1.0),
                (1.0, -1.0),
                (0.0, 0.0),
            ] {
                let point = point_in_view(&camera, depth, x, y);
                let [nx, ny, nz] = project(cascades.view_proj[index], point);
                assert!(
                    nx.abs() <= 1.0 && ny.abs() <= 1.0,
                    "cascade {index} {nx} {ny}"
                );
                assert!((0.0..=1.0).contains(&nz), "cascade {index} depth {nz}");
            }
        }
        start = end;
    }
}

#[test]
fn farther_cascades_cover_more_world_per_texel() {
    let cascades = ShadowCascades::fit(&camera_at(Vec3::ZERO, 0.0), sun(), TILE);
    let texel = cascades.bias_per_texel;
    assert!(texel.windows(2).all(|pair| pair[0] < pair[1]), "{texel:?}");
}

#[test]
fn turning_the_camera_keeps_the_volume_size() {
    let a = ShadowCascades::fit(&camera_at(Vec3::ZERO, 0.0), sun(), TILE);
    let b = ShadowCascades::fit(&camera_at(Vec3::ZERO, 2.1), sun(), TILE);
    for (x, y) in a.bias_per_texel.into_iter().zip(b.bias_per_texel) {
        assert!((x - y).abs() < 1e-9, "{x} != {y}");
    }
}

#[test]
fn moving_the_camera_moves_the_volume_by_whole_texels() {
    let at = |x: f32| ShadowCascades::fit(&camera_at(Vec3::new(x, 1.0, 0.0), 0.4), sun(), TILE);
    let probe = Vec3::new(0.37, 0.11, -0.52);
    for step in [0.013, 0.2, 1.7] {
        let (a, b) = (at(0.0), at(step));
        for index in 0..SHADOW_CASCADES {
            let [ax, ay, _] = project(a.view_proj[index], probe);
            let [bx, by, _] = project(b.view_proj[index], probe);
            let texels = |n: f32| n * TILE_TEXELS * 0.5;
            // Both frames place the probe at the same fractional texel position.
            let fraction = |n: f32| (texels(n) - texels(n).round()).abs();
            assert!(
                (fraction(ax) - fraction(bx)).abs() < 0.02,
                "cascade {index} step {step}"
            );
            assert!(
                (fraction(ay) - fraction(by)).abs() < 0.02,
                "cascade {index} step {step}"
            );
        }
    }
}

#[test]
fn an_orthographic_camera_gets_the_fixed_volume_in_every_cascade() {
    let camera = Camera {
        projection: Projection::orthographic((-5.0, 5.0), (-5.0, 5.0), (0.1, 50.0)),
        ..Camera::default()
    };
    let cascades = ShadowCascades::fit(&camera, sun(), TILE);
    assert_eq!(cascades, ShadowCascades::fixed(sun(), TILE));
    assert!(
        cascades
            .view_proj
            .iter()
            .all(|m| *m == cascades.view_proj[0])
    );
    assert!(cascades.far_edges.iter().all(|&edge| edge > 1e30));
}

#[test]
fn the_fixed_volume_keeps_the_single_map_bias() {
    let cascades = ShadowCascades::fixed(sun(), 2048);
    // 3.3 texels of a 40-unit volume over an 80-unit depth range, as before cascades.
    let base = 3.3 * cascades.bias_per_texel[0];
    assert!((base - 0.0008).abs() < 1e-5, "{base}");
}
