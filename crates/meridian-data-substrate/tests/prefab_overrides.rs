//! Field-level overrides of a prefab instance: merged into the prefab's
//! components, aimed at the entities the prefab made, and written next to
//! the whole-component replace that stays as it was.

mod common;

use common::{Follow, Motion, RigidBody, key, registry, transform};
use meridian_data_substrate::{LoadError, NoResolver, Parent, PrefabLibrary, PrefabPath, SceneMap};
use meridian_ecs::World;
use meridian_gac::{Motor3, Vec3};

fn path(s: &str) -> PrefabPath {
    PrefabPath::new(s).unwrap()
}

const HERO: &str = r#"{
  format: 1,
  entities: [
    { id: "body", name: "Body", components: {
        transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 1, 0] } },
        rigid_body: { motion: "dynamic", mass: 80 } } },
    { id: "gun", parent: "@entity:body", components: {
        transform: { motor: { rotor: [1, 0, 0, 0], translation: [0.5, 0, 0] } },
        follow: { target: "@entity:body" } } },
  ],
}"#;

fn library() -> PrefabLibrary {
    let mut lib = PrefabLibrary::new();
    lib.insert(path("actors/hero"), HERO);
    lib
}

fn load(scene: &str) -> Result<(World, SceneMap), LoadError> {
    let mut world = World::new();
    let map = registry().load_scene_with_prefabs(
        scene.as_bytes(),
        &mut world,
        &mut NoResolver,
        &mut library(),
    )?;
    Ok((world, map))
}

fn body(world: &World, map: &SceneMap, k: &str) -> RigidBody {
    world
        .get::<RigidBody>(map.entity(&key(k)).unwrap())
        .unwrap()
        .clone()
}

fn instance(overrides: &str) -> String {
    format!(
        r#"{{ format: 1, entities: [
            {{ id: "h", prefab: "@prefab:actors/hero", overrides: {overrides} }} ] }}"#
    )
}

#[test]
fn a_field_override_merges_into_the_prefabs_component() {
    let (world, map) = load(&instance(
        r#"{ h: { fields: {
            rigid_body: { mass: 2 },
            transform: { motor: { translation: [9, 0, 0] } } } } }"#,
    ))
    .unwrap();
    // The field is the instance's, the rest of the component the prefab's.
    let want = RigidBody {
        motion: Motion::Dynamic,
        mass: 2.0,
    };
    assert_eq!(body(&world, &map, "h"), want);
    assert_eq!(
        transform(&world, &map, "h"),
        Motor3::translation(Vec3::new(9.0, 0.0, 0.0))
    );
}

#[test]
fn an_override_may_name_a_generated_entity_by_its_path() {
    let (world, map) = load(&instance(
        r#"{ "h/gun": { fields: {
            transform: { motor: { translation: [1, 2, 3] } } } } }"#,
    ))
    .unwrap();
    assert_eq!(
        transform(&world, &map, "h/gun"),
        Motor3::translation(Vec3::new(1.0, 2.0, 3.0))
    );
    // Nothing else moved.
    assert_eq!(
        transform(&world, &map, "h"),
        Motor3::translation(Vec3::new(0.0, 1.0, 0.0))
    );
    let gun = map.entity(&key("h/gun")).unwrap();
    assert_eq!(world.get::<Follow>(gun).unwrap().target, key("h"));
}

#[test]
fn components_in_an_override_replace_the_whole_and_may_add() {
    let (world, map) = load(&instance(
        r#"{ "h/gun": { components: {
            rigid_body: { motion: "static", mass: 3 },
            follow: { target: "@entity:h/gun" } } } }"#,
    ))
    .unwrap();
    let want = RigidBody {
        motion: Motion::Static,
        mass: 3.0,
    };
    assert_eq!(body(&world, &map, "h/gun"), want);
    let gun = map.entity(&key("h/gun")).unwrap();
    assert_eq!(world.get::<Follow>(gun).unwrap().target, key("h/gun"));
}

#[test]
fn remove_takes_a_component_away() {
    let (world, map) = load(&instance(r#"{ "h/gun": { remove: ["follow"] } }"#)).unwrap();
    let gun = map.entity(&key("h/gun")).unwrap();
    assert!(world.get::<Follow>(gun).is_none());
    assert!(world.get::<Parent>(gun).is_some());
}

#[test]
fn the_whole_component_replace_of_the_entity_still_works_beside_overrides() {
    let (world, map) = load(
        r#"{ format: 1, entities: [
            { id: "h", prefab: "@prefab:actors/hero",
              components: { rigid_body: { motion: "static", mass: 1 } },
              overrides: { h: { fields: { rigid_body: { mass: 7 } } } } } ] }"#,
    )
    .unwrap();
    // Replaced whole first, then the field applied on top.
    let want = RigidBody {
        motion: Motion::Static,
        mass: 7.0,
    };
    assert_eq!(body(&world, &map, "h"), want);
}

#[test]
fn overrides_apply_in_the_order_written() {
    let (world, map) = load(&instance(
        r#"{ h: { components: { rigid_body: { motion: "static", mass: 1 } },
                  fields: { rigid_body: { mass: 5 } } } }"#,
    ))
    .unwrap();
    let want = RigidBody {
        motion: Motion::Static,
        mass: 5.0,
    };
    assert_eq!(body(&world, &map, "h"), want);
}

#[test]
fn overrides_reach_into_nested_instances() {
    let mut lib = library();
    lib.insert(
        path("squads/pair"),
        r#"{ format: 1, entities: [
            { id: "root" },
            { id: "leader", parent: "@entity:root", prefab: "@prefab:actors/hero",
              overrides: { leader: { fields: { rigid_body: { mass: 1 } } } } } ] }"#,
    );
    let scene = r#"{ format: 1, entities: [
        { id: "squad", prefab: "@prefab:squads/pair",
          overrides: { "squad/leader/gun": { fields: {
              follow: { target: "@entity:squad" } } } } } ] }"#;
    let mut world = World::new();
    let map = registry()
        .load_scene_with_prefabs(scene.as_bytes(), &mut world, &mut NoResolver, &mut lib)
        .unwrap();
    // The prefab's own override (written with its local ids) and the scene's.
    assert_eq!(body(&world, &map, "squad/leader").mass, 1.0);
    let gun = map.entity(&key("squad/leader/gun")).unwrap();
    assert_eq!(world.get::<Follow>(gun).unwrap().target, key("squad"));
}

#[test]
fn a_scene_entity_may_be_parented_to_a_generated_one() {
    let scene = r#"{ format: 1, entities: [
        { id: "h", prefab: "@prefab:actors/hero" },
        { id: "holster", parent: "@entity:h/gun",
          components: { follow: { target: "@entity:h/gun" } } } ] }"#;
    let (world, map) = load(scene).unwrap();
    let (holster, gun) = (
        map.entity(&key("holster")).unwrap(),
        map.entity(&key("h/gun")).unwrap(),
    );
    assert_eq!(world.get::<Parent>(holster), Some(&Parent(gun)));
    assert_eq!(world.get::<Follow>(holster).unwrap().target, key("h/gun"));
    assert_eq!(map.get(&key("holster")).unwrap().parent, Some(key("h/gun")));
}

#[test]
fn an_instance_nested_in_a_prefab_may_have_a_generated_parent() {
    let mut lib = library();
    lib.insert(
        path("squads/pair"),
        r#"{ format: 1, entities: [
            { id: "root" },
            { id: "leader", parent: "@entity:root", prefab: "@prefab:actors/hero" },
            { id: "pet", parent: "@entity:leader/gun" } ] }"#,
    );
    let mut world = World::new();
    let map = registry()
        .load_scene_with_prefabs(
            br#"{ format: 1, entities: [ { id: "s", prefab: "@prefab:squads/pair" } ] }"#,
            &mut world,
            &mut NoResolver,
            &mut lib,
        )
        .unwrap();
    let (pet, gun) = (
        map.entity(&key("s/pet")).unwrap(),
        map.entity(&key("s/leader/gun")).unwrap(),
    );
    assert_eq!(world.get::<Parent>(pet), Some(&Parent(gun)));
}

#[test]
fn overrides_in_binary_are_the_same_document() {
    let text = instance(
        r#"{ "h/gun": { fields: { transform: { motor: { translation: [4, 0, 0] } } } } }"#,
    );
    let parsed = substrate_data::text::parse(&text).unwrap();
    let bytes = substrate_data::binary::encode(&parsed.value);
    let mut world = World::new();
    let map = registry()
        .load_scene_with_prefabs(&bytes, &mut world, &mut NoResolver, &mut library())
        .unwrap();
    assert_eq!(
        transform(&world, &map, "h/gun"),
        Motor3::translation(Vec3::new(4.0, 0.0, 0.0))
    );
}
