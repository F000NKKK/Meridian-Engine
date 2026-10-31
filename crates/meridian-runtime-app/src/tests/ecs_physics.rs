//! Real collision and synchronization through independently attached components.
use crate::{PhysicsProblem, resolve_spatial_components, step_ecs_physics};
use meridian_ecs::{Position, Transform, World};
use meridian_gac::{Motor3, Vec3};
use meridian_physics_app::{
    Collider, ColliderShape, PhysicsBody, PhysicsSubsystem, SequentialStepper,
};

fn falling_sphere() -> (World, meridian_ecs::Entity) {
    let mut world = World::new();
    let floor = world.spawn();
    world.insert(
        floor,
        Transform {
            motor: Motor3::translation(Vec3::new(0.0, -0.5, 0.0)),
        },
    );
    world.insert(
        floor,
        Collider(ColliderShape::Cuboid {
            half_extents: Vec3::new(10.0, 0.5, 10.0),
        }),
    );
    let ball = world.spawn();
    world.insert(ball, Position(Vec3::Y * 3.0));
    world.insert(ball, Collider(ColliderShape::Sphere { radius: 0.5 }));
    world.insert(
        ball,
        PhysicsBody {
            mass: 1.0,
            ..PhysicsBody::default()
        },
    );
    (world, ball)
}

#[test]
fn a_dynamic_sphere_lands_on_a_static_collider_without_pose_reset() {
    let (mut world, ball) = falling_sphere();
    let mut physics = PhysicsSubsystem::default();
    for _ in 0..240 {
        let issues = step_ecs_physics(&mut world, &mut physics, &mut SequentialStepper, 1.0 / 60.0)
            .expect("valid step");
        assert_eq!(issues, Vec::new());
        assert_eq!(resolve_spatial_components(&mut world), Vec::new());
    }
    let y = world
        .get::<Transform>(ball)
        .expect("simulated")
        .motor
        .transform_point(Vec3::ZERO)
        .y;
    assert!(
        (0.45..0.55).contains(&y),
        "sphere should rest on the floor, y={y}"
    );
    assert!(
        (world
            .get::<Position>(ball)
            .expect("source synchronized")
            .0
            .y
            - y)
            .abs()
            < 1e-5
    );
    world.despawn(ball);
    step_ecs_physics(&mut world, &mut physics, &mut SequentialStepper, 1.0 / 60.0)
        .expect("remove body");
    assert_eq!(physics.bodies.len(), 1);
}

#[test]
fn invalid_step_does_not_mutate_the_world_or_the_body_list() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position(Vec3::Y));
    let mut physics = PhysicsSubsystem::default();
    for dt in [0.0, -1.0, f32::NAN] {
        assert_eq!(
            step_ecs_physics(&mut world, &mut physics, &mut SequentialStepper, dt),
            Err(PhysicsProblem::Parameters)
        );
        assert!(world.get::<Transform>(entity).is_none());
    }
}

#[test]
fn parented_and_invalid_shapes_are_excluded_with_entity_diagnostics() {
    let (mut world, ball) = falling_sphere();
    let parent = world.spawn();
    world.insert(ball, meridian_ecs::Parent(parent));
    let mut physics = PhysicsSubsystem::default();
    let issues =
        step_ecs_physics(&mut world, &mut physics, &mut SequentialStepper, 0.01).expect("step");
    assert_eq!(issues[0].entity, ball);
    assert_eq!(issues[0].problem, PhysicsProblem::ParentOrScale);
    assert_eq!(physics.bodies.len(), 1);
    world.remove::<meridian_ecs::Parent>(ball);
    world.insert(ball, Collider(ColliderShape::Sphere { radius: -1.0 }));
    let issues =
        step_ecs_physics(&mut world, &mut physics, &mut SequentialStepper, 0.01).expect("step");
    assert_eq!(issues[0].problem, PhysicsProblem::Parameters);
}

#[test]
fn motor_authoring_receives_the_simulated_pose() {
    let (mut world, ball) = falling_sphere();
    world.remove::<Position>(ball);
    world.insert(ball, Motor3::translation(Vec3::Y * 3.0));
    let mut physics = PhysicsSubsystem::default();
    assert_eq!(
        step_ecs_physics(&mut world, &mut physics, &mut SequentialStepper, 0.1).expect("step"),
        Vec::new()
    );
    let source = *world.get::<Motor3>(ball).expect("motor remains attached");
    assert!(source.transform_point(Vec3::ZERO).y < 3.0);
    assert_eq!(world.get::<Transform>(ball).expect("derived").motor, source);
}
