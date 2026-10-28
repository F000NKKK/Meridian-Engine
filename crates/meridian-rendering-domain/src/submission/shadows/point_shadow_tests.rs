//! Tests of the point light shadow cubes (ADR 041, ADR 043).

use meridian_gac::{Motor3, Vec3};

use super::tests::project;
use super::*;

fn first_point(scene: &crate::Scene3d) -> Option<PointShadow> {
    match LocalShadow::nearest(scene)[0] {
        Some(LocalShadow::Point(point)) => Some(point),
        _ => None,
    }
}

fn scene_with_point_light(position: Vec3, range: f32) -> crate::Scene3d {
    crate::Scene3d {
        lights: vec![
            crate::Light::Directional {
                direction: Motor3::identity(),
                color: [1.0; 3],
                intensity: 1.0,
            },
            crate::Light::Point {
                position: Motor3::translation(position),
                color: [1.0; 3],
                intensity: 1.0,
                range,
            },
        ],
        ..crate::Scene3d::default()
    }
}

#[test]
fn a_point_light_has_six_faces_that_each_see_their_own_axis() {
    let center = Vec3::new(1.0, 2.0, 3.0);
    let point = first_point(&scene_with_point_light(center, 20.0)).expect("a point light");
    assert_eq!(point.caster, 1);
    assert_eq!(point.position, center);
    let axes = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
    for (face, axis) in axes.into_iter().enumerate() {
        let [x, y, z] = project(point.view_proj[face], center + axis * 5.0);
        assert!(x.abs() < 1e-4 && y.abs() < 1e-4, "face {face}: ({x}, {y})");
        assert!((0.0..1.0).contains(&z), "face {face}: depth {z}");
    }
}

#[test]
fn a_corner_direction_stays_inside_the_face_it_belongs_to() {
    let point = first_point(&scene_with_point_light(Vec3::ZERO, 20.0)).expect("a point light");
    let [x, y, _] = project(point.view_proj[0], Vec3::new(5.0, 4.9, -4.9));
    assert!(x.abs() <= 1.0 && y.abs() <= 1.0, "({x}, {y})");
}

#[test]
fn a_scene_without_a_point_light_or_with_no_range_has_no_point_shadow() {
    assert!(first_point(&crate::Scene3d::default()).is_none());
    assert!(first_point(&scene_with_point_light(Vec3::ZERO, 0.01)).is_none());
}

fn point_light_at(x: f32) -> crate::Light {
    crate::Light::Point {
        position: Motor3::translation(Vec3::new(x, 0.0, 0.0)),
        color: [1.0; 3],
        intensity: 1.0,
        range: 10.0,
    }
}

#[test]
fn the_nearest_point_lights_come_first_and_the_rest_are_dropped() {
    let positions = [40.0, 5.0, 30.0, 10.0, 20.0, 1.0];
    let scene = crate::Scene3d {
        lights: positions.map(point_light_at).to_vec(),
        ..crate::Scene3d::default()
    };
    let near: Vec<f32> = LocalShadow::nearest(&scene)
        .iter()
        .map(|point| point.expect("four of six").position().x)
        .collect();
    assert_eq!(near, [1.0, 5.0, 10.0, 20.0]);
    let indices: Vec<u32> = LocalShadow::nearest(&scene)
        .iter()
        .flatten()
        .map(LocalShadow::caster)
        .collect();
    assert_eq!(indices, [5, 1, 3, 4], "each keeps its light index");
}

#[test]
fn fewer_point_lights_than_the_budget_leave_the_slots_empty() {
    let scene = crate::Scene3d {
        lights: vec![point_light_at(2.0)],
        ..crate::Scene3d::default()
    };
    let slots = LocalShadow::nearest(&scene);
    assert!(slots[0].is_some());
    assert!(slots[1..].iter().all(Option::is_none));
}
