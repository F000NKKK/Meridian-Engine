//! Tests of the float flavor's projection matrices.

use core::f32::consts::PI;

use meridian_numeric::Scalar;

use crate::*;

#[test]
fn perspective_projects_frustum_boundary_points_to_ndc_edges() {
    let fov_y = PI / 2.0; // 90 degrees
    let aspect = 16.0 / 9.0;
    let near = 0.1;
    let far = 100.0;
    let proj = Projection::perspective(fov_y, aspect, near, far);

    let depth = 10.0_f32;
    let half_height = depth * (fov_y * 0.5).tan();
    let half_width = half_height * aspect;

    let project = |p: [Scalar; 4]| -> [Scalar; 4] {
        let m = proj.0;
        [
            m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0] * p[3],
            m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1] * p[3],
            m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2] * p[3],
            m[0][3] * p[0] + m[1][3] * p[1] + m[2][3] * p[2] + m[3][3] * p[3],
        ]
    };

    let top_edge = project([0.0, half_height, -depth, 1.0]);
    assert!((top_edge[1] / top_edge[3] - 1.0).abs() < 1e-4);
    let right_edge = project([half_width, 0.0, -depth, 1.0]);
    assert!((right_edge[0] / right_edge[3] - 1.0).abs() < 1e-4);

    let at_near = project([0.0, 0.0, -near, 1.0]);
    assert!(
        (at_near[2] / at_near[3]).abs() < 1e-5,
        "near plane maps to depth 0"
    );
    let at_far = project([0.0, 0.0, -far, 1.0]);
    assert!(
        (at_far[2] / at_far[3] - 1.0).abs() < 1e-5,
        "far plane maps to depth 1"
    );
}

#[test]
fn orthographic_maps_box_corners_to_ndc_cube() {
    let proj = Projection::orthographic(-2.0, 2.0, -1.0, 1.0, 0.5, 10.0);
    let m = proj.0;
    let project = |p: [Scalar; 4]| -> [Scalar; 4] {
        [
            m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0] * p[3],
            m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1] * p[3],
            m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2] * p[3],
            m[0][3] * p[0] + m[1][3] * p[1] + m[2][3] * p[2] + m[3][3] * p[3],
        ]
    };
    // Orthographic w stays 1: no perspective divide needed.
    let near_corner = project([-2.0, -1.0, -0.5, 1.0]);
    assert!((near_corner[0] - -1.0).abs() < 1e-5);
    assert!((near_corner[1] - -1.0).abs() < 1e-5);
    assert!((near_corner[2] - 0.0).abs() < 1e-5);
    let far_corner = project([2.0, 1.0, -10.0, 1.0]);
    assert!((far_corner[0] - 1.0).abs() < 1e-5);
    assert!((far_corner[1] - 1.0).abs() < 1e-5);
    assert!((far_corner[2] - 1.0).abs() < 1e-5);
}
