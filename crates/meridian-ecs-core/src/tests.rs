//! Tests of this module.

use std::any::TypeId;

use meridian_gac_core::Motor3;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Position(f32, f32);
impl Component for Position {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Velocity(f32, f32);
impl Component for Velocity {}

#[test]
fn spawn_creates_alive_entity() {
    let mut world = World::new();
    let e = world.spawn();
    assert!(world.is_alive(e));
}

#[test]
fn despawn_kills_entity() {
    let mut world = World::new();
    let e = world.spawn();
    assert!(world.despawn(e));
    assert!(!world.is_alive(e));
    assert!(
        !world.despawn(e),
        "double despawn must return false, not panic"
    );
}

#[test]
fn insert_and_get_roundtrip() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0, 2.0));
    assert_eq!(world.get::<Position>(e), Some(&Position(1.0, 2.0)));
}

#[test]
fn insert_on_existing_component_overwrites_in_place() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0, 2.0));
    world.insert(e, Position(9.0, 9.0));
    assert_eq!(world.get::<Position>(e), Some(&Position(9.0, 9.0)));
}

#[test]
fn insert_moves_entity_across_archetypes_preserving_existing_components() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0, 2.0));
    world.insert(e, Velocity(3.0, 4.0));

    // The archetype move (Position-only -> Position+Velocity) must not
    // lose or corrupt Position's already-stored value.
    assert_eq!(world.get::<Position>(e), Some(&Position(1.0, 2.0)));
    assert_eq!(world.get::<Velocity>(e), Some(&Velocity(3.0, 4.0)));
}

#[test]
fn remove_takes_value_out_and_entity_survives() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0, 2.0));
    world.insert(e, Velocity(3.0, 4.0));

    assert_eq!(world.remove::<Velocity>(e), Some(Velocity(3.0, 4.0)));
    assert!(world.is_alive(e));
    assert_eq!(world.get::<Velocity>(e), None);
    assert_eq!(
        world.get::<Position>(e),
        Some(&Position(1.0, 2.0)),
        "removing Velocity must not disturb Position"
    );
}

#[test]
fn remove_missing_component_returns_none() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0, 2.0));
    assert_eq!(world.remove::<Velocity>(e), None);
}

#[test]
fn dead_or_unknown_entity_lookup_returns_none_not_panic() {
    let world = World::new();
    let fake = Entity {
        index: 999,
        generation: 0,
    };
    assert_eq!(world.get::<Position>(fake), None);
}

#[test]
fn despawn_swap_remove_fixes_up_displaced_entity_location() {
    let mut world = World::new();
    let a = world.spawn();
    world.insert(a, Position(1.0, 1.0));
    let b = world.spawn();
    world.insert(b, Position(2.0, 2.0));
    let c = world.spawn();
    world.insert(c, Position(3.0, 3.0));

    // Despawning `a` (row 0) swap-removes the last row (c) into its
    // place; `b` must still resolve correctly afterward.
    world.despawn(a);
    assert_eq!(world.get::<Position>(b), Some(&Position(2.0, 2.0)));
    assert_eq!(world.get::<Position>(c), Some(&Position(3.0, 3.0)));
}

#[test]
fn query_spans_every_archetype_containing_the_component() {
    let mut world = World::new();
    let a = world.spawn();
    world.insert(a, Position(1.0, 0.0));

    let b = world.spawn();
    world.insert(b, Position(2.0, 0.0));
    world.insert(b, Velocity(0.0, 0.0)); // b lives in a different archetype than a

    let mut seen: Vec<(Entity, Position)> =
        world.query::<Position>().map(|(e, p)| (e, *p)).collect();
    seen.sort_by_key(|(e, _)| e.index);

    assert_eq!(seen, vec![(a, Position(1.0, 0.0)), (b, Position(2.0, 0.0))]);
}

#[test]
fn query_mut_allows_in_place_mutation() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Position(1.0, 1.0));

    for (_, pos) in world.query_mut::<Position>() {
        pos.0 += 10.0;
    }

    assert_eq!(world.get::<Position>(e), Some(&Position(11.0, 1.0)));
}

#[test]
fn transform_is_a_component_and_defaults_to_identity_motor() {
    let mut world = World::new();
    let e = world.spawn();
    world.insert(e, Transform::default());
    assert_eq!(world.get::<Transform>(e).unwrap().motor, Motor3::identity());
}

#[test]
fn entities_lists_every_live_entity() {
    let mut world = World::new();
    let a = world.spawn();
    let b = world.spawn();
    let c = world.spawn();
    world.despawn(b);
    let mut listed: Vec<_> = world.entities().collect();
    listed.sort_by_key(|e| e.index);
    assert_eq!(listed, vec![a, c]);
}

#[test]
fn component_types_lists_the_entitys_components_and_tracks_changes() {
    let mut world = World::new();
    let e = world.spawn();
    assert_eq!(world.component_types(e), Some(&[][..]));

    world.insert(e, Position(1.0, 2.0));
    world.insert(e, Velocity(0.0, 1.0));
    let mut expected = vec![TypeId::of::<Position>(), TypeId::of::<Velocity>()];
    expected.sort();
    assert_eq!(world.component_types(e), Some(expected.as_slice()));

    world.remove::<Velocity>(e);
    assert_eq!(
        world.component_types(e),
        Some(&[TypeId::of::<Position>()][..])
    );
}

#[test]
fn component_types_is_none_for_a_dead_entity() {
    let mut world = World::new();
    let e = world.spawn();
    world.despawn(e);
    assert_eq!(world.component_types(e), None);
}
