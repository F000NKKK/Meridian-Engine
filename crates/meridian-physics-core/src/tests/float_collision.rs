//! Float-flavor tests of the collision engine: broad phase and narrow phase.

use meridian_gac_core::{Motor3, Vec3};

use super::float_fixtures::{assert_vec3_approx, cuboid, sphere};
use crate::float::*;
use core::f32::consts::PI;

#[test]
fn aabb_overlap_basic_cases() {
    use meridian_gac_core::Aabb;
    let a = Aabb::from_sphere(Vec3::ZERO, 1.0);
    let touching = Aabb::from_sphere(Vec3::new(2.0, 0.0, 0.0), 1.0);
    let separate = Aabb::from_sphere(Vec3::new(10.0, 0.0, 0.0), 1.0);
    assert!(
        a.overlaps(&touching),
        "boxes sharing a boundary count as overlapping"
    );
    assert!(!a.overlaps(&separate));
}

#[test]
fn broad_phase_finds_only_overlapping_pairs() {
    let bodies = vec![
        sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0),
        sphere(Vec3::new(1.5, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0), // overlaps body 0
        sphere(Vec3::new(100.0, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0), // far away
    ];
    let mut broad = BroadPhase::new();
    let pairs = broad.find_candidate_pairs(&bodies);
    assert_eq!(pairs, &[(0, 1)]);
}

#[test]
fn narrow_phase_reports_normal_pointing_from_a_to_b_and_correct_penetration() {
    let bodies = vec![
        sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0),
        sphere(Vec3::new(1.5, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0),
    ];
    let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
    assert_eq!(contact.normal, Vec3::new(1.0, 0.0, 0.0));
    assert!(
        (contact.penetration - 0.5).abs() < 1e-5,
        "spheres of radius 1 each, centers 1.5 apart -> 0.5 overlap"
    );
}

#[test]
fn narrow_phase_returns_none_when_not_touching() {
    let bodies = vec![
        sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0),
        sphere(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0),
    ];
    assert!(NarrowPhase::new().test_pair(&bodies, 0, 1).is_none());
}

#[test]
fn narrow_phase_sphere_vs_cuboid_reports_correct_penetration_and_normal() {
    let floor = cuboid(
        Motor3::translation(Vec3::new(0.0, -5.0, 0.0)),
        Vec3::ZERO,
        0.0,
        Vec3::new(10.0, 5.0, 10.0),
    );
    let ball = sphere(Vec3::new(0.0, 0.3, 0.0), Vec3::ZERO, 1.0, 0.5);
    let bodies = vec![floor, ball];

    let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
    assert_vec3_approx(contact.normal, Vec3::new(0.0, 1.0, 0.0));
    assert!((contact.penetration - 0.2).abs() < 1e-4);
    assert_vec3_approx(contact.point, Vec3::ZERO);
}

#[test]
fn narrow_phase_sphere_vs_cuboid_returns_none_when_separated() {
    let floor = cuboid(
        Motor3::identity(),
        Vec3::ZERO,
        0.0,
        Vec3::new(1.0, 1.0, 1.0),
    );
    let ball = sphere(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO, 1.0, 0.5);
    let bodies = vec![floor, ball];
    assert!(NarrowPhase::new().test_pair(&bodies, 0, 1).is_none());
}

#[test]
fn narrow_phase_sphere_vs_cuboid_handles_embedded_sphere_without_panicking() {
    let box_body = cuboid(
        Motor3::identity(),
        Vec3::ZERO,
        0.0,
        Vec3::new(1.0, 1.0, 1.0),
    );
    let ball = sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 0.3);
    let bodies = vec![box_body, ball];

    let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
    assert!(contact.penetration > 0.0 && contact.penetration.is_finite());
    assert!((contact.normal.length() - 1.0).abs() < 1e-4);
}

#[test]
fn narrow_phase_cuboid_vs_cuboid_reports_least_penetration_axis() {
    let a = cuboid(
        Motor3::identity(),
        Vec3::ZERO,
        1.0,
        Vec3::new(1.0, 1.0, 1.0),
    );
    let b = cuboid(
        Motor3::translation(Vec3::new(1.5, 0.0, 0.0)),
        Vec3::ZERO,
        1.0,
        Vec3::new(1.0, 1.0, 1.0),
    );
    let bodies = vec![a, b];

    let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
    assert_vec3_approx(contact.normal, Vec3::new(1.0, 0.0, 0.0));
    assert!((contact.penetration - 0.5).abs() < 1e-4);
}

#[test]
fn narrow_phase_cuboid_vs_cuboid_returns_none_when_separated() {
    let a = cuboid(
        Motor3::identity(),
        Vec3::ZERO,
        1.0,
        Vec3::new(1.0, 1.0, 1.0),
    );
    let b = cuboid(
        Motor3::translation(Vec3::new(5.0, 0.0, 0.0)),
        Vec3::ZERO,
        1.0,
        Vec3::new(1.0, 1.0, 1.0),
    );
    let bodies = vec![a, b];
    assert!(NarrowPhase::new().test_pair(&bodies, 0, 1).is_none());
}

#[test]
fn narrow_phase_cuboid_vs_cuboid_handles_rotated_boxes() {
    let a = cuboid(
        Motor3::identity(),
        Vec3::ZERO,
        1.0,
        Vec3::new(1.0, 1.0, 1.0),
    );
    let overlapping_b = cuboid(
        Motor3::rotation(Vec3::Y, PI / 4.0).compose(Motor3::translation(Vec3::new(1.5, 0.0, 0.0))),
        Vec3::ZERO,
        1.0,
        Vec3::new(1.0, 1.0, 1.0),
    );
    let bodies = vec![a, overlapping_b];
    let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
    assert!(contact.penetration > 0.0 && contact.penetration.is_finite());
    assert!((contact.normal.length() - 1.0).abs() < 1e-4);

    let separated_b = cuboid(
        Motor3::rotation(Vec3::Y, PI / 4.0).compose(Motor3::translation(Vec3::new(10.0, 0.0, 0.0))),
        Vec3::ZERO,
        1.0,
        Vec3::new(1.0, 1.0, 1.0),
    );
    let bodies = vec![a, separated_b];
    assert!(NarrowPhase::new().test_pair(&bodies, 0, 1).is_none());
}
