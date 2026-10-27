//! Float-flavor tests of the collision engine: broad phase and narrow phase.

use core::f32::consts::PI;

use meridian_gac_core::{Motor3, Vec3};

use super::float_fixtures::{assert_vec3_approx, cuboid, sphere};
use crate::float::*;

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

/// The x-axis sweep must report exactly the pairs an exhaustive sweep does,
/// in the same order, including for bodies listed out of x order and for
/// bodies sharing a boundary.
#[test]
fn broad_phase_sweep_matches_exhaustive_pair_search() {
    let mut seed = 0x2545_F491_u32;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f32::from(u16::try_from(seed >> 16).expect("16 bits")) / 65_536.0
    };
    let bodies: Vec<_> = (0..120)
        .map(|_| {
            let position = Vec3::new(next() * 30.0, next() * 6.0, next() * 6.0);
            sphere(position, Vec3::ZERO, 1.0, 1.0)
        })
        .collect();
    let mut expected = Vec::new();
    for i in 0..bodies.len() {
        for j in (i + 1)..bodies.len() {
            if bodies[i].aabb().overlaps(&bodies[j].aabb()) {
                expected.push((i, j));
            }
        }
    }
    let mut broad = BroadPhase::new();
    assert_eq!(broad.find_candidate_pairs(&bodies), expected.as_slice());
    assert!(!expected.is_empty(), "scenario must contain overlaps");
}

#[test]
fn broad_phase_keeps_pairs_that_only_touch_along_x() {
    let bodies = vec![
        sphere(Vec3::new(2.0, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0),
        sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0),
    ];
    let mut broad = BroadPhase::new();
    assert_eq!(broad.find_candidate_pairs(&bodies), &[(0, 1)]);
}
