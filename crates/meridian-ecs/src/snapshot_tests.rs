//! World snapshots: copy, roll back, and what must survive the round trip.

use std::any::TypeId;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Position(f32, f32);
impl Component for Position {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Velocity(f32, f32);
impl Component for Velocity {}

struct Opaque;
impl Component for Opaque {}

fn cloners() -> Cloners {
    Cloners::new().with::<Position>().with::<Velocity>()
}

#[test]
fn restore_brings_back_values_and_removes_later_entities() {
    let mut world = World::new();
    let kept = world.spawn();
    world.insert(kept, Position(1.0, 2.0));
    world.insert(kept, Velocity(3.0, 4.0));
    let snapshot = world.snapshot(&cloners()).expect("all types registered");
    assert_eq!(snapshot.entity_count(), 1);

    world.insert(kept, Position(9.0, 9.0));
    world.remove::<Velocity>(kept);
    let later = world.spawn();
    world.insert(later, Position(5.0, 5.0));

    world.restore(snapshot);

    assert_eq!(world.get::<Position>(kept), Some(&Position(1.0, 2.0)));
    assert_eq!(world.get::<Velocity>(kept), Some(&Velocity(3.0, 4.0)));
    assert!(!world.is_alive(later));
    assert_eq!(world.entities().count(), 1);
}

#[test]
fn restore_resurrects_an_entity_with_the_same_generation() {
    let mut world = World::new();
    let doomed = world.spawn();
    world.insert(doomed, Position(1.0, 1.0));
    let snapshot = world.snapshot(&cloners()).expect("registered");

    world.despawn(doomed);
    let reused = world.spawn();
    assert_eq!(reused.index, doomed.index);
    assert_ne!(reused.generation, doomed.generation);

    world.restore(snapshot);

    assert!(
        world.is_alive(doomed),
        "the old handle names the entity again"
    );
    assert_eq!(world.get::<Position>(doomed), Some(&Position(1.0, 1.0)));
    assert!(!world.is_alive(reused));
}

#[test]
fn a_snapshot_is_independent_of_the_world_it_came_from() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0, 1.0));
    let snapshot = world.snapshot(&cloners()).expect("registered");

    for (_, position) in world.query_mut::<Position>() {
        position.0 = 100.0;
    }
    world.restore(snapshot);

    assert_eq!(world.get::<Position>(e), Some(&Position(1.0, 1.0)));
}

#[test]
fn an_unregistered_component_type_is_reported_and_nothing_is_copied() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(0.0, 0.0));
    world.insert(e, Opaque);

    let error = world.snapshot(&cloners()).expect_err("Opaque is unknown");

    assert_eq!(
        error,
        SnapshotError::Unregistered(vec![TypeId::of::<Opaque>()])
    );
}

#[test]
fn restoring_marks_everything_changed_for_change_detection() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0, 1.0));
    let seen_until = world.advance_tick();
    let snapshot = world.snapshot(&cloners()).expect("registered");
    for (_, position) in world.query_mut::<Position>() {
        position.0 = 2.0;
    }
    let before = world.tick();

    world.restore(snapshot);

    assert!(world.tick() > before, "the clock never runs backwards");
    let changed = world
        .query_many_since::<(&Position, Changed<Position>)>(seen_until)
        .count();
    assert_eq!(changed, 1);
}

#[test]
fn an_empty_world_round_trips() {
    let mut world = World::new();
    let snapshot = world.snapshot(&Cloners::new()).expect("nothing to clone");
    let e = world.spawn();
    world.restore(snapshot);
    assert!(!world.is_alive(e));
    assert_eq!(world.entities().count(), 0);
}
