//! Tests of this module.

use meridian_gac::{Motor3, Rotor, Scale3, Vec3};

use super::*;
use crate::{Changed, Entity, Transform, World};

fn at(x: f32, y: f32, z: f32) -> Transform {
    Transform {
        motor: Motor3::translation(Vec3::new(x, y, z)),
    }
}

fn spawn(world: &mut World, transform: Transform, parent: Option<Entity>) -> Entity {
    let entity = world.spawn();
    world.insert(entity, transform);
    if let Some(parent) = parent {
        world.insert(entity, Parent(parent));
    }
    entity
}

fn global_point(world: &World, entity: Entity, point: Vec3) -> Vec3 {
    world
        .get::<GlobalTransform>(entity)
        .expect("propagated")
        .motor
        .transform_point(point)
}

fn assert_near(a: Vec3, b: Vec3) {
    assert!((a - b).length() < 1e-4, "{a:?} != {b:?}");
}

#[test]
fn a_root_keeps_its_local_motor() {
    let mut world = World::new();
    let root = spawn(&mut world, at(1.0, 2.0, 3.0), None);

    assert_eq!(propagate_transforms(&mut world), 1);

    assert_near(
        global_point(&world, root, Vec3::ZERO),
        Vec3::new(1.0, 2.0, 3.0),
    );
}

#[test]
fn a_chain_applies_the_local_motor_first_then_each_ancestor() {
    let mut world = World::new();
    let turned = Transform {
        motor: Motor3::from_rotation_translation(
            Rotor::from_axis_angle(Vec3::Z, std::f32::consts::FRAC_PI_2),
            Vec3::new(10.0, 0.0, 0.0),
        ),
    };
    let root = spawn(&mut world, turned, None);
    let middle = spawn(&mut world, at(0.0, 2.0, 0.0), Some(root));
    let leaf = spawn(&mut world, at(1.0, 0.0, 0.0), Some(middle));

    propagate_transforms(&mut world);

    let p = Vec3::new(0.5, 0.25, 0.0);
    let by_steps = turned.motor.transform_point(
        at(0.0, 2.0, 0.0)
            .motor
            .transform_point(at(1.0, 0.0, 0.0).motor.transform_point(p)),
    );
    assert_near(global_point(&world, leaf, p), by_steps);
}

#[test]
fn the_result_does_not_depend_on_spawn_order() {
    let mut world = World::new();
    let leaf = world.spawn();
    let root = spawn(&mut world, at(5.0, 0.0, 0.0), None);
    world.insert(leaf, at(0.0, 1.0, 0.0));
    world.insert(leaf, Parent(root));

    propagate_transforms(&mut world);

    assert_near(
        global_point(&world, leaf, Vec3::ZERO),
        Vec3::new(5.0, 1.0, 0.0),
    );
}

#[test]
fn a_despawned_parent_makes_its_child_a_root() {
    let mut world = World::new();
    let parent = spawn(&mut world, at(5.0, 0.0, 0.0), None);
    let child = spawn(&mut world, at(0.0, 1.0, 0.0), Some(parent));
    world.despawn(parent);

    propagate_transforms(&mut world);

    assert_near(
        global_point(&world, child, Vec3::ZERO),
        Vec3::new(0.0, 1.0, 0.0),
    );
}

#[test]
fn a_parent_without_a_transform_makes_its_child_a_root() {
    let mut world = World::new();
    let bare = world.spawn();
    let child = spawn(&mut world, at(0.0, 1.0, 0.0), Some(bare));

    propagate_transforms(&mut world);

    assert_near(
        global_point(&world, child, Vec3::ZERO),
        Vec3::new(0.0, 1.0, 0.0),
    );
}

#[test]
fn members_of_a_cycle_are_roots_and_their_descendants_follow() {
    let mut world = World::new();
    let a = spawn(&mut world, at(1.0, 0.0, 0.0), None);
    let b = spawn(&mut world, at(0.0, 2.0, 0.0), Some(a));
    world.insert(a, Parent(b));
    let below = spawn(&mut world, at(0.0, 0.0, 4.0), Some(a));

    propagate_transforms(&mut world);

    assert_near(
        global_point(&world, a, Vec3::ZERO),
        Vec3::new(1.0, 0.0, 0.0),
    );
    assert_near(
        global_point(&world, b, Vec3::ZERO),
        Vec3::new(0.0, 2.0, 0.0),
    );
    assert_near(
        global_point(&world, below, Vec3::ZERO),
        Vec3::new(1.0, 0.0, 4.0),
    );
}

#[test]
fn an_entity_that_is_its_own_parent_is_a_root() {
    let mut world = World::new();
    let ouroboros = spawn(&mut world, at(3.0, 0.0, 0.0), None);
    world.insert(ouroboros, Parent(ouroboros));

    propagate_transforms(&mut world);

    assert_near(
        global_point(&world, ouroboros, Vec3::ZERO),
        Vec3::new(3.0, 0.0, 0.0),
    );
}

#[test]
fn a_second_pass_with_nothing_moved_writes_nothing() {
    let mut world = World::new();
    let root = spawn(&mut world, at(1.0, 0.0, 0.0), None);
    spawn(&mut world, at(0.0, 1.0, 0.0), Some(root));
    assert_eq!(propagate_transforms(&mut world), 2);

    assert_eq!(propagate_transforms(&mut world), 0);
}

#[test]
fn moving_a_parent_moves_its_children_and_only_them() {
    let mut world = World::new();
    let mover = spawn(&mut world, at(1.0, 0.0, 0.0), None);
    let child = spawn(&mut world, at(0.0, 1.0, 0.0), Some(mover));
    let bystander = spawn(&mut world, at(9.0, 9.0, 9.0), None);
    propagate_transforms(&mut world);

    world
        .get_mut::<Transform>(mover)
        .expect("has transform")
        .motor = Motor3::translation(Vec3::new(7.0, 0.0, 0.0));

    assert_eq!(propagate_transforms(&mut world), 2);
    assert_near(
        global_point(&world, child, Vec3::ZERO),
        Vec3::new(7.0, 1.0, 0.0),
    );
    assert_near(
        global_point(&world, bystander, Vec3::ZERO),
        Vec3::new(9.0, 9.0, 9.0),
    );
}

#[test]
fn a_pass_that_moves_nothing_leaves_changed_filters_empty() {
    let mut world = World::new();
    let root = spawn(&mut world, at(1.0, 0.0, 0.0), None);
    spawn(&mut world, at(0.0, 1.0, 0.0), Some(root));
    propagate_transforms(&mut world);
    let since = world.advance_tick();

    propagate_transforms(&mut world);

    let changed = world
        .query_many_since::<(&GlobalTransform, Changed<GlobalTransform>)>(since)
        .count();
    assert_eq!(changed, 0);
}

#[test]
fn reparenting_changes_the_global_motor() {
    let mut world = World::new();
    let left = spawn(&mut world, at(-5.0, 0.0, 0.0), None);
    let right = spawn(&mut world, at(5.0, 0.0, 0.0), None);
    let child = spawn(&mut world, at(0.0, 1.0, 0.0), Some(left));
    propagate_transforms(&mut world);

    world.insert(child, Parent(right));
    propagate_transforms(&mut world);

    assert_near(
        global_point(&world, child, Vec3::ZERO),
        Vec3::new(5.0, 1.0, 0.0),
    );
}

fn scaled(world: &mut World, entity: Entity, x: f32, y: f32, z: f32) {
    world.insert(entity, Scale3::new(x, y, z).expect("a valid scale"));
}

fn global_scale(world: &World, entity: Entity) -> [f32; 3] {
    world
        .get::<GlobalTransform>(entity)
        .expect("propagated")
        .scale
        .components()
}

#[test]
fn scales_multiply_down_the_chain_and_stretch_the_child_placement() {
    let mut world = World::new();
    let root = spawn(&mut world, at(0.0, 0.0, 0.0), None);
    scaled(&mut world, root, 2.0, 2.0, 2.0);
    let child = spawn(&mut world, at(1.0, 0.0, 0.0), Some(root));
    scaled(&mut world, child, 3.0, 1.0, 1.0);
    propagate_transforms(&mut world);

    assert_eq!(global_scale(&world, child), [6.0, 2.0, 2.0]);
    assert_near(
        global_point(&world, child, Vec3::ZERO),
        Vec3::new(2.0, 0.0, 0.0),
    );
}

#[test]
fn a_turned_scaled_parent_places_the_child_along_its_own_axes() {
    let mut world = World::new();
    let turned = Transform {
        motor: Motor3::from_rotation_translation(
            Rotor::from_axis_angle(Vec3::Z, std::f32::consts::FRAC_PI_2),
            Vec3::new(10.0, 0.0, 0.0),
        ),
    };
    let root = spawn(&mut world, turned, None);
    scaled(&mut world, root, 2.0, 1.0, 1.0);
    let child = spawn(&mut world, at(1.0, 0.0, 0.0), Some(root));
    propagate_transforms(&mut world);

    // One unit along the parent's x, stretched to two, then turned onto +y.
    assert_near(
        global_point(&world, child, Vec3::ZERO),
        Vec3::new(10.0, 2.0, 0.0),
    );
}

#[test]
fn an_unscaled_child_of_a_scaled_parent_is_scaled_with_it() {
    let mut world = World::new();
    let root = spawn(&mut world, at(0.0, 0.0, 0.0), None);
    scaled(&mut world, root, 2.0, 2.0, 2.0);
    let child = spawn(&mut world, at(0.0, 1.0, 0.0), Some(root));
    propagate_transforms(&mut world);

    assert_eq!(global_scale(&world, child), [2.0; 3]);
    assert_near(
        global_point(&world, child, Vec3::ZERO),
        Vec3::new(0.0, 2.0, 0.0),
    );
}

#[test]
fn an_unscaled_chain_has_the_unit_scale_and_a_new_scale_marks_it_changed() {
    let mut world = World::new();
    let root = spawn(&mut world, at(1.0, 0.0, 0.0), None);
    propagate_transforms(&mut world);
    assert_eq!(global_scale(&world, root), [1.0; 3]);
    assert_eq!(propagate_transforms(&mut world), 0);

    scaled(&mut world, root, 1.0, 2.0, 1.0);
    assert_eq!(propagate_transforms(&mut world), 1);
    assert_eq!(global_scale(&world, root), [1.0, 2.0, 1.0]);
}
