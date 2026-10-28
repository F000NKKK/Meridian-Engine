//! Tests of optional elements and the `With` / `Without` filters.

use super::fixtures::{Mass, Position, Tag, Velocity, spawn_moving};
use crate::{Entity, With, Without, World};

#[test]
fn optional_element_matches_entities_without_the_component() {
    let mut world = World::new();
    let heavy = spawn_moving(&mut world, 1.0, 1.0);
    world.insert(heavy, Mass(5.0));
    let light = spawn_moving(&mut world, 2.0, 2.0);

    let mut rows: Vec<_> = world
        .query_many::<(&Position, Option<&Mass>)>()
        .map(|(e, (_, mass))| (e, mass.copied()))
        .collect();
    rows.sort_by_key(|row| row.0.index);

    assert_eq!(rows, vec![(heavy, Some(Mass(5.0))), (light, None)]);
}

#[test]
fn optional_mutable_element_writes_only_where_present() {
    let mut world = World::new();
    let heavy = spawn_moving(&mut world, 1.0, 1.0);
    world.insert(heavy, Mass(5.0));
    let light = spawn_moving(&mut world, 2.0, 2.0);

    for (_, (_, mass)) in world.query_many_mut::<(&Position, Option<&mut Mass>)>() {
        if let Some(mass) = mass {
            mass.0 *= 2.0;
        }
    }

    assert_eq!(world.get::<Mass>(heavy), Some(&Mass(10.0)));
    assert_eq!(world.get::<Mass>(light), None);
}

#[test]
fn optional_element_alone_with_a_required_one_still_requires_the_required() {
    let mut world = World::new();
    let only_mass = world.spawn();
    world.insert(only_mass, Mass(1.0));

    let count = world.query_many::<(&Position, Option<&Mass>)>().count();

    assert_eq!(count, 0);
}

#[test]
fn without_filter_drops_entities_that_have_the_component() {
    let mut world = World::new();
    let free = spawn_moving(&mut world, 1.0, 1.0);
    let tagged = spawn_moving(&mut world, 2.0, 2.0);
    world.insert(tagged, Tag);

    let found: Vec<Entity> = world
        .query_many::<(&Position, Without<Tag>)>()
        .map(|(e, _)| e)
        .collect();

    assert_eq!(found, vec![free]);
}

#[test]
fn with_filter_keeps_only_entities_that_have_the_component() {
    let mut world = World::new();
    spawn_moving(&mut world, 1.0, 1.0);
    let tagged = spawn_moving(&mut world, 2.0, 2.0);
    world.insert(tagged, Tag);

    let found: Vec<Entity> = world
        .query_many::<(&Position, With<Tag>)>()
        .map(|(e, _)| e)
        .collect();

    assert_eq!(found, vec![tagged]);
}

#[test]
fn filter_on_a_component_that_is_also_written_does_not_panic() {
    let mut world = World::new();
    let e = spawn_moving(&mut world, 1.0, 1.0);

    for (_, (position, ())) in world.query_many_mut::<(&mut Position, With<Position>)>() {
        position.0 += 1.0;
    }

    assert_eq!(world.get::<Position>(e), Some(&Position(2.0)));
}

#[test]
fn mutable_query_with_filter_and_optional_mixes_all_element_kinds() {
    let mut world = World::new();
    let moving = spawn_moving(&mut world, 1.0, 3.0);
    world.insert(moving, Mass(2.0));
    let tagged = spawn_moving(&mut world, 5.0, 7.0);
    world.insert(tagged, Tag);

    for (_, (position, velocity, mass, ())) in
        world.query_many_mut::<(&mut Position, &Velocity, Option<&Mass>, Without<Tag>)>()
    {
        position.0 += velocity.0 * mass.map_or(1.0, |m| m.0);
    }

    assert_eq!(world.get::<Position>(moving), Some(&Position(7.0)));
    assert_eq!(world.get::<Position>(tagged), Some(&Position(5.0)));
}
