//! `Changed<T>` and the world clock.

use super::fixtures::{Position, Velocity};
use crate::{Changed, Tick, World};

fn moved(world: &World, since: Tick) -> usize {
    world
        .query_many_since::<(&Position, Changed<Position>)>(since)
        .count()
}

#[test]
fn a_fresh_world_reports_every_written_column_as_changed() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0));
    assert_eq!(moved(&world, Tick::ZERO), 1);
    assert_eq!(
        world.query_many::<(&Position, Changed<Position>)>().count(),
        1
    );
}

#[test]
fn nothing_is_changed_after_the_system_advanced_the_clock() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0));
    let seen = world.advance_tick();
    assert_eq!(moved(&world, seen), 0);
}

#[test]
fn mutable_access_by_entity_marks_the_column_changed() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0));
    let seen = world.advance_tick();
    assert_eq!(moved(&world, seen), 0);
    world.get_mut::<Position>(e).unwrap().0 = 6.0;
    assert_eq!(moved(&world, seen), 1);
}

#[test]
fn insert_over_an_existing_component_marks_it_changed() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0));
    let seen = world.advance_tick();
    world.insert(e, Position(2.0));
    assert_eq!(moved(&world, seen), 1);
}

#[test]
fn a_mutable_query_marks_only_the_columns_it_writes() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0));
    world.insert(e, Velocity(1.0));
    let seen = world.advance_tick();

    for (_, (position, _velocity)) in world.query_many_mut::<(&mut Position, &Velocity)>() {
        position.0 += 1.0;
    }

    assert_eq!(moved(&world, seen), 1);
    let velocity_changed = world
        .query_many_since::<(&Velocity, Changed<Velocity>)>(seen)
        .count();
    assert_eq!(velocity_changed, 0);
}

#[test]
fn changed_is_per_archetype_not_per_entity() {
    let mut world = World::new();
    let still = world.spawn();
    world.insert(still, Position(0.0));
    let moving = world.spawn();
    world.insert(moving, Position(0.0));
    let seen = world.advance_tick();
    world.get_mut::<Position>(moving).unwrap().0 = 3.0;
    assert_eq!(moved(&world, seen), 2, "both share one archetype column");
}

#[test]
fn archetype_moves_and_despawns_count_as_changes() {
    let mut world = World::new();
    let a = world.spawn();
    world.insert(a, Position(1.0));
    let b = world.spawn();
    world.insert(b, Position(2.0));
    let seen = world.advance_tick();
    assert_eq!(moved(&world, seen), 0);

    world.despawn(b);
    assert_eq!(moved(&world, seen), 1);

    let before_insert = world.advance_tick();
    world.insert(a, Velocity(1.0));
    let after_insert = world
        .query_many_since::<(&Position, &Velocity, Changed<Position>)>(before_insert)
        .count();
    assert_eq!(after_insert, 1);
}

#[test]
fn writes_after_a_system_advanced_the_clock_are_never_missed() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0));
    // A system runs, then advances the clock for the next run.
    let last_run = world.advance_tick();
    // Another system writes before the frame ends.
    world.get_mut::<Position>(e).unwrap().0 = 2.0;
    world.advance_tick();
    assert_eq!(moved(&world, last_run), 1);
}
