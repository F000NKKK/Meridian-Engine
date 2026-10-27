//! Tests of this module.

use core::f32::consts::PI;

use super::*;
use crate::generic::Shape;

fn assert_vec3_approx(a: Vec3, b: Vec3) {
    assert!(
        a.approx_eq(b),
        "expected {b:?} to approximately equal {a:?}"
    );
}

/// Independent oracle: Rodrigues' rotation formula, computed with
/// plain vector arithmetic (no geometric algebra), to cross-check
/// `Rotor`/`Motor3` against a ground truth that doesn't share any
/// code path with the PGA implementation.
fn rodrigues(v: Vec3, axis: Vec3, angle: Scalar) -> Vec3 {
    let n = axis.normalize();
    v * angle.cos() + n.cross(v) * angle.sin() + n * (n.dot(v) * (1.0 - angle.cos()))
}

#[test]
fn rotor_matches_rodrigues_oracle() {
    let axis = Vec3::new(0.3, -0.7, 1.2);
    let angle = 1.234;
    let v = Vec3::new(1.0, 2.0, -3.0);

    let rotor = Rotor::from_axis_angle(axis, angle);
    let got = rotor.transform_vector(v);
    let want = rodrigues(v, axis, angle);
    assert_vec3_approx(got, want);
}

#[test]
fn rotor_about_z_is_right_handed() {
    let rotor = Rotor::from_axis_angle(Vec3::Z, PI / 2.0);
    let got = rotor.transform_vector(Vec3::X);
    assert_vec3_approx(got, Vec3::Y);
}

#[test]
fn rotor_composition_adds_angles() {
    let axis = Vec3::new(0.2, 0.4, 0.9);
    let a = Rotor::from_axis_angle(axis, 0.5);
    let b = Rotor::from_axis_angle(axis, 0.8);
    let composed = a.compose(b);
    let direct = Rotor::from_axis_angle(axis, 1.3);

    let v = Vec3::new(1.0, 0.5, -0.25);
    assert_vec3_approx(composed.transform_vector(v), direct.transform_vector(v));
}

#[test]
fn translation_is_additive() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(-0.5, 4.0, 0.25);
    let p = Vec3::new(10.0, -5.0, 2.0);

    let composed = Motor3::translation(a).compose(Motor3::translation(b));
    let direct = Motor3::translation(a + b);

    assert_vec3_approx(composed.transform_point(p), direct.transform_point(p));
    assert_vec3_approx(direct.transform_point(p), p + a + b);
}

#[test]
fn motor_matches_rotate_then_translate_oracle() {
    let axis = Vec3::new(-0.4, 0.1, 0.9);
    let angle = 0.9;
    let t = Vec3::new(2.0, -1.0, 0.5);
    let p = Vec3::new(1.0, 0.0, 0.0);

    let motor = Motor3::from_rotation_translation(Rotor::from_axis_angle(axis, angle), t);
    let got = motor.transform_point(p);

    // independent oracle: rotate via Rodrigues, then translate
    let want = rodrigues(p, axis, angle) + t;

    assert_vec3_approx(got, want);
}

#[test]
fn motor_inverse_round_trips() {
    let motor = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::new(0.3, 0.6, -0.2), 0.77),
        Vec3::new(3.0, -2.0, 1.0),
    );
    let p = Vec3::new(5.0, 6.0, 7.0);

    let round_tripped = motor.inverse().transform_point(motor.transform_point(p));
    assert_vec3_approx(round_tripped, p);
}

/// The roadmap milestone: parent/child transform hierarchy propagation.
/// Transforming a point through the child's local motor and then the
/// parent's motor must equal transforming it through the single
/// composed world motor — this is the whole reason `Transform`
/// composition is "one multiplication" instead of a position-plus-
/// rotation merge (see docs/gac-design.md).
#[test]
fn parent_child_hierarchy_composition() {
    let parent = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::Z, PI / 4.0),
        Vec3::new(10.0, 0.0, 0.0),
    );
    let child = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::Y, PI / 6.0),
        Vec3::new(0.0, 2.0, 0.0),
    );
    let local_point = Vec3::new(1.0, 0.0, 0.0);

    let world_motor = child.compose(parent);
    let via_composed_motor = world_motor.transform_point(local_point);

    let via_step_by_step = parent.transform_point(child.transform_point(local_point));

    assert_vec3_approx(via_composed_motor, via_step_by_step);
}

#[test]
fn bivector_exp_matches_rotor_from_axis_angle_directly() {
    let axis = Vec3::new(0.2, -0.5, 0.8);
    let angle = 1.1;
    let unit_axis = axis.normalize();
    let bivector = Bivector3::new(unit_axis.x, unit_axis.y, unit_axis.z) * angle;

    let via_exp = bivector.exp();
    let via_direct = Rotor::from_axis_angle(axis, angle);

    let p = Vec3::new(1.0, 2.0, 3.0);
    assert_vec3_approx(via_exp.transform_vector(p), via_direct.transform_vector(p));
}

#[test]
fn bivector_exp_of_zero_is_identity() {
    let rotor = Bivector3::ZERO.exp();
    let p = Vec3::new(1.0, 2.0, 3.0);
    assert_vec3_approx(rotor.transform_vector(p), p);
}

#[test]
fn integrating_constant_angular_velocity_matches_one_big_rotation() {
    // Spinning at a constant angular velocity for `steps` small dt's
    // and composing the per-step rotors must match one direct
    // rotation by the total angle swept — the same "many small
    // compositions = one big one" property `rotor_composition_adds_angles`
    // checks, but exercised through the exponential-map integration
    // path a physics integrator actually uses.
    let angular_velocity = Bivector3::new(0.0, 0.0, 3.0); // "about Z", magnitude 3 rad/s
    let dt = 0.001;
    let steps = 1000;

    let mut accumulated = Rotor::identity();
    for _ in 0..steps {
        let step_rotor = (angular_velocity * dt).exp();
        accumulated = accumulated.compose(step_rotor);
    }

    let direct = Rotor::from_axis_angle(Vec3::Z, angular_velocity.length() * dt * steps as f32);

    // A looser tolerance than assert_vec3_approx's 1e-5 is correct
    // here, not a workaround: this test measures f32 accumulation
    // error over 1000 sequential compositions, which is a real
    // property of the floating-point path, not the algebra being
    // wrong. 1000 steps at 1kHz is already more substeps than a
    // physics integrator runs per frame at 60-120Hz.
    let p = Vec3::X;
    let got = accumulated.transform_vector(p);
    let want = direct.transform_vector(p);
    assert!(
        (got - want).length() < 5e-4,
        "accumulated drift too large: got {got:?}, want {want:?}"
    );
}

#[test]
fn wedge_matches_cross_product_components() {
    let a = Vec3::new(1.0, 0.0, 0.0);
    let b = Vec3::new(0.0, 1.0, 0.0);
    let bivector = Bivector3::wedge(a, b);
    let cross = a.cross(b);
    assert!((bivector.e23 - cross.x).abs() < 1e-6);
    assert!((bivector.e31 - cross.y).abs() < 1e-6);
    assert!((bivector.e12 - cross.z).abs() < 1e-6);
}

fn mat4_mul_point(m: [[Scalar; 4]; 4], p: Vec3) -> Vec3 {
    // Column-major, column-vector convention: v' = M * [p, 1]^T.
    let x = m[0][0] * p.x + m[1][0] * p.y + m[2][0] * p.z + m[3][0];
    let y = m[0][1] * p.x + m[1][1] * p.y + m[2][1] * p.z + m[3][1];
    let z = m[0][2] * p.x + m[1][2] * p.y + m[2][2] * p.z + m[3][2];
    Vec3::new(x, y, z)
}

#[test]
fn to_mat4_reproduces_transform_point_for_translation() {
    let motor = Motor3::translation(Vec3::new(1.0, 2.0, 3.0));
    let p = Vec3::new(5.0, -1.0, 0.5);
    assert_vec3_approx(mat4_mul_point(motor.to_mat4(), p), motor.transform_point(p));
}

#[test]
fn to_mat4_reproduces_transform_point_for_rotation_and_translation() {
    let motor = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::Z, PI / 3.0),
        Vec3::new(-2.0, 4.0, 1.0),
    );
    for p in [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(2.0, -3.0, 7.0),
    ] {
        assert_vec3_approx(mat4_mul_point(motor.to_mat4(), p), motor.transform_point(p));
    }
}

#[test]
fn motor_transform_vector_ignores_translation() {
    let motor = Motor3::translation(Vec3::new(100.0, -50.0, 25.0));
    // A pure translation has no rotational part: every direction must
    // come back unchanged.
    for v in [Vec3::X, Vec3::Y, Vec3::new(1.0, 2.0, 3.0)] {
        assert_vec3_approx(motor.transform_vector(v), v);
    }
}

#[test]
fn motor_transform_vector_matches_rotor_for_pure_rotation_plus_translation() {
    let rotor = Rotor::from_axis_angle(Vec3::Z, PI / 3.0);
    let motor = Motor3::from_rotation_translation(rotor, Vec3::new(7.0, -3.0, 2.0));
    let v = Vec3::new(1.0, 2.0, -1.0);
    // Translation must cancel out of transform_vector regardless of
    // how large it is, leaving exactly the rotor's own action.
    assert_vec3_approx(motor.transform_vector(v), rotor.transform_vector(v));
}

#[test]
fn to_mat4_identity_is_the_identity_matrix() {
    let m = Motor3::identity().to_mat4();
    let mut expected = [[0.0; 4]; 4];
    for (i, row) in expected.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    assert_eq!(m, expected);
}

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

#[test]
fn point_support_is_always_itself() {
    let p = Vec3::new(3.0, -2.0, 7.0);
    assert_eq!(p.support(Vec3::X), p);
    assert_eq!(p.support(-Vec3::Z), p);
}

#[test]
fn sphere_support_is_center_plus_radius_along_direction() {
    let sphere = Sphere {
        center: Vec3::new(1.0, 2.0, 3.0),
        radius: 5.0,
    };
    assert_vec3_approx(sphere.support(Vec3::X), Vec3::new(6.0, 2.0, 3.0));
    assert_vec3_approx(sphere.support(-Vec3::X), Vec3::new(-4.0, 2.0, 3.0));
}

#[test]
fn sphere_support_degenerate_direction_returns_center() {
    // Vec3::normalize's documented behavior for a near-zero vector is
    // to return it unchanged, so a zero query direction degenerates to
    // "no displacement" rather than a divide-by-zero.
    let sphere = Sphere {
        center: Vec3::new(1.0, 2.0, 3.0),
        radius: 5.0,
    };
    assert_vec3_approx(sphere.support(Vec3::ZERO), sphere.center);
}

#[test]
fn aabb_cube_has_equal_half_extents() {
    let cube = Aabb::cube(Vec3::new(1.0, 1.0, 1.0), 2.0);
    assert_vec3_approx(cube.min, Vec3::new(-1.0, -1.0, -1.0));
    assert_vec3_approx(cube.max, Vec3::new(3.0, 3.0, 3.0));
}

#[test]
fn aabb_support_matches_frustum_style_positive_vertex() {
    let aabb = Aabb {
        min: Vec3::new(-1.0, -2.0, -3.0),
        max: Vec3::new(4.0, 5.0, 6.0),
    };
    assert_vec3_approx(aabb.support(Vec3::new(1.0, 1.0, 1.0)), aabb.max);
    assert_vec3_approx(aabb.support(Vec3::new(-1.0, -1.0, -1.0)), aabb.min);
}

#[test]
fn obb_with_identity_frame_matches_aabb_support() {
    let center = Vec3::new(2.0, 0.0, -1.0);
    let half_extents = Vec3::new(1.0, 2.0, 3.0);
    let obb = Obb {
        frame: Motor3::translation(center),
        half_extents,
    };
    let aabb = Aabb {
        min: center - half_extents,
        max: center + half_extents,
    };
    for direction in [Vec3::X, -Vec3::X, Vec3::Y, Vec3::new(1.0, 1.0, 1.0)] {
        assert_vec3_approx(obb.support(direction), aabb.support(direction));
    }
}

#[test]
fn obb_rotated_90_degrees_about_z_swaps_x_and_y_extents() {
    // A box rotated 90 degrees about Z: its local +X half-extent
    // (3.0) now points along world +Y, so querying support along
    // world +Y must reach out by 3.0, not the local y half-extent
    // (1.0). Checking only the Y component deliberately: a query
    // exactly along one local axis lands on a box *edge* (every
    // combination of the other two axes' signs is an equally valid
    // support point there), so the other components are a tie-break
    // detail, not a claim this test should make.
    let obb = Obb {
        frame: Motor3::rotation(Vec3::Z, PI / 2.0),
        half_extents: Vec3::new(3.0, 1.0, 1.0),
    };
    assert!((obb.support(Vec3::Y).y - 3.0).abs() < 1e-4);
}

#[test]
fn obb_cube_has_equal_half_extents() {
    let obb = Obb::cube(Motor3::identity(), 2.0);
    assert_vec3_approx(obb.half_extents, Vec3::new(2.0, 2.0, 2.0));
}

fn test_cone() -> Cone {
    // Apex at the origin, opening toward +Z, half-angle 45 degrees
    // (tan == 1, so base_radius == height — easy to check by hand).
    Cone {
        apex: Vec3::ZERO,
        axis: Vec3::Z,
        half_angle: PI / 4.0,
        height: 2.0,
    }
}

#[test]
fn cone_support_backward_along_axis_is_the_apex() {
    let cone = test_cone();
    assert_vec3_approx(cone.support(-Vec3::Z), Vec3::ZERO);
}

#[test]
fn cone_support_forward_along_axis_is_on_the_base_rim() {
    let cone = test_cone();
    // Zero lateral component: Cone::support's degenerate branch
    // returns the base center itself, which is the farthest point
    // straight down the axis.
    assert_vec3_approx(cone.support(Vec3::Z), Vec3::new(0.0, 0.0, 2.0));
}

#[test]
fn cone_support_sideways_reaches_the_base_rim_not_the_apex() {
    let cone = test_cone();
    // base_radius = height * tan(45deg) = 2.0, so straight sideways
    // (+X) the support point is the rim point (2, 0, 2), clearly
    // farther along +X than the apex at the origin.
    assert_vec3_approx(cone.support(Vec3::X), Vec3::new(2.0, 0.0, 2.0));
}

#[test]
fn plane_contains_uses_the_shapes_best_case_point() {
    // Inside is x >= 0.
    let plane = Plane {
        normal: Vec3::X,
        d: 0.0,
    };
    assert!(plane.contains(&Vec3::new(5.0, 0.0, 0.0)));
    assert!(!plane.contains(&Vec3::new(-5.0, 0.0, 0.0)));

    // A sphere straddling the plane (center just behind it, radius
    // large enough to poke through) must count as "contained" —
    // conservative, not "fully inside".
    let straddling = Sphere {
        center: Vec3::new(-0.5, 0.0, 0.0),
        radius: 1.0,
    };
    assert!(plane.contains(&straddling));

    // A sphere entirely on the excluded side must not.
    let excluded = Sphere {
        center: Vec3::new(-5.0, 0.0, 0.0),
        radius: 1.0,
    };
    assert!(!plane.contains(&excluded));
}

#[test]
fn convex_volume_generalizes_intersects_to_any_shape() {
    // A 2x2x2 cube volume centered at the origin, one plane per face,
    // each normal pointing inward.
    let volume = ConvexVolume::new(vec![
        Plane {
            normal: Vec3::X,
            d: 1.0,
        },
        Plane {
            normal: -Vec3::X,
            d: 1.0,
        },
        Plane {
            normal: Vec3::Y,
            d: 1.0,
        },
        Plane {
            normal: -Vec3::Y,
            d: 1.0,
        },
        Plane {
            normal: Vec3::Z,
            d: 1.0,
        },
        Plane {
            normal: -Vec3::Z,
            d: 1.0,
        },
    ]);

    assert!(volume.intersects(&Sphere {
        center: Vec3::ZERO,
        radius: 0.5,
    }));
    assert!(!volume.intersects(&Sphere {
        center: Vec3::new(10.0, 0.0, 0.0),
        radius: 0.5,
    }));
    assert!(volume.intersects(&Aabb::cube(Vec3::new(0.9, 0.0, 0.0), 0.5)));
    assert!(volume.intersects(&Obb::cube(Motor3::identity(), 0.5)));
}

#[test]
fn icosphere_vertices_lie_on_the_unit_sphere() {
    let mesh = icosphere(1);
    for v in &mesh.vertices {
        assert!(
            (v.length() - 1.0).abs() < 1e-4,
            "every icosphere vertex must be unit length, got {v:?} (length {})",
            v.length()
        );
    }
}

#[test]
fn icosphere_unique_edges_has_no_duplicate_or_self_edges() {
    let mesh = icosphere(1);
    let edges = mesh.unique_edges();
    for &(a, b) in &edges {
        assert_ne!(a, b, "an edge must connect two distinct vertices");
        assert!(a < mesh.vertices.len() && b < mesh.vertices.len());
    }
    let mut sorted = edges.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        edges.len(),
        "unique_edges must not report the same edge twice"
    );
}

#[test]
fn icosphere_subdivision_increases_vertex_and_face_count() {
    let base = icosphere(0);
    let subdivided = icosphere(1);
    assert_eq!(base.vertices.len(), 12);
    assert_eq!(base.faces.len(), 20);
    assert!(subdivided.vertices.len() > base.vertices.len());
    assert_eq!(subdivided.faces.len(), base.faces.len() * 4);
}
