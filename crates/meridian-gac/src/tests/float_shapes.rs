//! Tests of the float flavor's shapes, volumes and meshes.

use core::f32::consts::PI;

use crate::*;

fn assert_vec3_approx(a: Vec3, b: Vec3) {
    assert!(
        a.approx_eq(b),
        "expected {b:?} to approximately equal {a:?}"
    );
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

#[test]
fn obb_support_picks_the_positive_corner_for_a_zero_component() {
    // Regression: `signum` made `-0.0` pick the negative corner, so the
    // result flipped with the sign of a zero the frame math happened to leave.
    let obb = Obb {
        frame: Motor3::identity(),
        half_extents: Vec3::new(1.0, 2.0, 3.0),
    };
    assert_vec3_approx(
        obb.support(Vec3::new(1.0, -0.0, -0.0)),
        Vec3::new(1.0, 2.0, 3.0),
    );
}
