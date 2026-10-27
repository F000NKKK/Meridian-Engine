//! Tests of `query_many` and `query_many_mut` over required components.

use super::fixtures::{Mass, Position, Tag, Velocity, spawn_moving};
use crate::{Entity, World};

#[test]
fn query_many_skips_entities_missing_a_component() {
    let mut world = World::new();
    let moving = spawn_moving(&mut world, 1.0, 2.0);
    let still = world.spawn();
    world.insert(still, Position(9.0));

    let rows: Vec<_> = world.query_many::<(&Position, &Velocity)>().collect();

    assert_eq!(rows, vec![(moving, (&Position(1.0), &Velocity(2.0)))]);
}

#[test]
fn query_many_spans_archetypes_with_extra_components() {
    let mut world = World::new();
    let plain = spawn_moving(&mut world, 1.0, 1.0);
    let tagged = spawn_moving(&mut world, 2.0, 2.0);
    world.insert(tagged, Tag);

    let mut found: Vec<Entity> = world
        .query_many::<(&Position, &Velocity)>()
        .map(|(e, _)| e)
        .collect();
    found.sort_by_key(|e| e.index);

    assert_eq!(found, vec![plain, tagged]);
}

#[test]
fn query_many_mut_writes_through_the_mutable_element() {
    let mut world = World::new();
    let e = spawn_moving(&mut world, 1.0, 0.5);

    for (_, (position, velocity)) in world.query_many_mut::<(&mut Position, &Velocity)>() {
        position.0 += velocity.0;
    }

    assert_eq!(world.get::<Position>(e), Some(&Position(1.5)));
    assert_eq!(world.get::<Velocity>(e), Some(&Velocity(0.5)));
}

#[test]
fn query_many_mut_matches_element_order_not_storage_order() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Velocity(3.0));
    world.insert(e, Position(4.0));

    let rows: Vec<_> = world
        .query_many_mut::<(&mut Velocity, &mut Position)>()
        .map(|(_, (v, p))| (v.0, p.0))
        .collect();

    assert_eq!(rows, vec![(3.0, 4.0)]);
}

#[test]
fn query_many_yields_three_and_four_element_tuples() {
    let mut world = World::new();
    let e = spawn_moving(&mut world, 1.0, 2.0);
    world.insert(e, Mass(3.0));
    world.insert(e, Tag);

    let three: Vec<_> = world
        .query_many::<(&Position, &Velocity, &Mass)>()
        .collect();
    let four: Vec<_> = world
        .query_many::<(&Position, &Velocity, &Mass, &Tag)>()
        .collect();

    assert_eq!(
        three,
        vec![(e, (&Position(1.0), &Velocity(2.0), &Mass(3.0)))]
    );
    assert_eq!(
        four,
        vec![(e, (&Position(1.0), &Velocity(2.0), &Mass(3.0), &Tag))]
    );
}

#[test]
fn query_many_after_despawn_keeps_rows_aligned() {
    let mut world = World::new();
    let first = spawn_moving(&mut world, 1.0, 10.0);
    let second = spawn_moving(&mut world, 2.0, 20.0);
    let third = spawn_moving(&mut world, 3.0, 30.0);
    world.despawn(first);

    let mut rows: Vec<_> = world
        .query_many::<(&Position, &Velocity)>()
        .map(|(e, (p, v))| (e, p.0, v.0))
        .collect();
    rows.sort_by_key(|row| row.0.index);

    assert_eq!(rows, vec![(second, 2.0, 20.0), (third, 3.0, 30.0)]);
}

#[test]
fn query_many_on_empty_world_yields_nothing() {
    let world = World::new();

    assert_eq!(world.query_many::<(&Position, &Velocity)>().count(), 0);
}

#[test]
fn query_many_mut_on_unmatched_component_yields_nothing() {
    let mut world = World::new();
    spawn_moving(&mut world, 1.0, 1.0);

    assert_eq!(
        world.query_many_mut::<(&mut Position, &mut Mass)>().count(),
        0
    );
}

#[test]
#[should_panic(expected = "appears twice")]
fn query_many_mut_with_a_repeated_component_panics() {
    let mut world = World::new();
    spawn_moving(&mut world, 1.0, 1.0);

    let _query = world.query_many_mut::<(&mut Position, &Position)>();
}
