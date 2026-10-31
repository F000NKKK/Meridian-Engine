//! Independent spatial components resolve without changing other component data.
use crate::{Shared, SpatialProblem, SpatialStage, Stage, resolve_spatial_components};
use meridian_ecs::{
    Component, GlobalTransform, Parent, Position, Transform, World, propagate_transforms,
};
use meridian_gac::{Motor3, Rotor, Vec3};

#[derive(Debug)]
struct AttachedCollider;
impl Component for AttachedCollider {}

#[test]
fn independent_rotation_and_position_feed_hierarchy_and_preserve_other_components() {
    let mut world = World::new();
    let parent = world.spawn();
    world.insert(parent, Motor3::translation(Vec3::X * 10.0));
    let child = world.spawn();
    world.insert(child, Parent(parent));
    world.insert(child, Position(Vec3::Y));
    world.insert(
        child,
        Rotor::from_axis_angle(Vec3::Z, std::f32::consts::FRAC_PI_2),
    );
    world.insert(child, AttachedCollider);
    assert_eq!(resolve_spatial_components(&mut world), Vec::new());
    propagate_transforms(&mut world);
    let pose = world
        .get::<GlobalTransform>(child)
        .expect("derived global")
        .motor;
    assert!((pose.transform_point(Vec3::X) - Vec3::new(10.0, 2.0, 0.0)).length() < 1e-5);
    assert!(world.get::<AttachedCollider>(child).is_some());
    world.get_mut::<Position>(child).expect("position").0 = Vec3::Y * 3.0;
    assert_eq!(resolve_spatial_components(&mut world), Vec::new());
    assert!(
        (world
            .get::<Transform>(child)
            .expect("updated")
            .motor
            .transform_point(Vec3::ZERO)
            - Vec3::Y * 3.0)
            .length()
            < 1e-5
    );
}

#[test]
fn competing_sources_and_invalid_numbers_report_without_overwriting_pose() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(
        entity,
        Transform {
            motor: Motor3::translation(Vec3::X),
        },
    );
    world.insert(entity, Motor3::identity());
    world.insert(entity, Rotor::identity());
    assert_eq!(
        resolve_spatial_components(&mut world)[0].problem,
        SpatialProblem::CompetingSources
    );
    assert_eq!(
        world
            .get::<Transform>(entity)
            .expect("unchanged")
            .motor
            .transform_point(Vec3::ZERO),
        Vec3::X
    );
    world.remove::<Motor3>(entity);
    world.insert(entity, Position(Vec3::new(f32::NAN, 0.0, 0.0)));
    assert_eq!(
        resolve_spatial_components(&mut world)[0].problem,
        SpatialProblem::NonFinite
    );
}

#[test]
fn runtime_diagnostics_remain_accessible_after_stage_is_moved() {
    let world = Shared::new(World::new());
    let entity = world.lock().spawn();
    world
        .lock()
        .insert(entity, Position(Vec3::new(f32::INFINITY, 0.0, 0.0)));
    let mut stage = SpatialStage::new(world.clone());
    let issues = stage.issues();
    stage.run();
    assert_eq!(issues.lock()[0].entity, entity);
    world.lock().insert(entity, Position(Vec3::ZERO));
    stage.run();
    assert!(issues.lock().is_empty());
}

#[test]
fn defaults_and_invalid_rotors_are_explicit_and_legacy_transforms_are_untouched() {
    let mut world = World::new();
    let position = world.spawn();
    world.insert(position, Position(Vec3::X));
    let rotation = world.spawn();
    world.insert(rotation, Rotor::identity());
    let legacy = world.spawn();
    world.insert(
        legacy,
        Transform {
            motor: Motor3::translation(Vec3::Y),
        },
    );
    assert_eq!(resolve_spatial_components(&mut world), Vec::new());
    assert_eq!(
        world
            .get::<Transform>(position)
            .expect("position only")
            .motor
            .transform_point(Vec3::ZERO),
        Vec3::X
    );
    assert_eq!(
        world
            .get::<Transform>(rotation)
            .expect("rotor only")
            .motor
            .transform_point(Vec3::ZERO),
        Vec3::ZERO
    );
    assert_eq!(
        world
            .get::<Transform>(legacy)
            .expect("legacy")
            .motor
            .transform_point(Vec3::ZERO),
        Vec3::Y
    );
    world.insert(rotation, Rotor(meridian_gac::Multivector([0.0; 16])));
    assert!(matches!(
        resolve_spatial_components(&mut world)[0].problem,
        SpatialProblem::Rotation(_)
    ));
    world.remove::<Rotor>(rotation);
    let mut invalid = Motor3::identity();
    invalid.0.0[0] = f32::NAN;
    world.insert(rotation, invalid);
    assert_eq!(
        resolve_spatial_components(&mut world)[0].problem,
        SpatialProblem::NonFinite
    );
}
