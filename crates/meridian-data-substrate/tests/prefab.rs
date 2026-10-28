//! `.mprefab`: instancing under ids `<instance>/<prefab id>`, whole-component
//! overrides and nesting.

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

fn load(scene: &str, lib: &mut PrefabLibrary) -> Result<(World, SceneMap), LoadError> {
    let mut world = World::new();
    let map =
        registry().load_scene_with_prefabs(scene.as_bytes(), &mut world, &mut NoResolver, lib)?;
    Ok((world, map))
}

fn keys(scene: &SceneMap) -> Vec<&str> {
    scene.entities().iter().map(|e| e.key.as_str()).collect()
}

#[test]
fn an_instance_is_the_scene_entity_with_the_prefab_under_it() {
    let scene = r#"{ format: 1, entities: [
        { id: "world" },
        { id: "hero", name: "Player", prefab: "@prefab:actors/hero", parent: "@entity:world",
          components: { transform: { motor: { rotor: [1, 0, 0, 0], translation: [9, 0, 0] } } } },
    ] }"#;
    let (world, map) = load(scene, &mut library()).unwrap();
    assert_eq!(keys(&map), ["world", "hero", "hero/gun"]);

    let hero = map.entity(&key("hero")).unwrap();
    let gun = map.entity(&key("hero/gun")).unwrap();
    // The prefab root's components are the base; a listed component replaces the whole.
    assert_eq!(
        world.get::<RigidBody>(hero),
        Some(&RigidBody {
            motion: Motion::Dynamic,
            mass: 80.0
        })
    );
    assert_eq!(
        transform(&world, &map, "hero"),
        Motor3::translation(Vec3::new(9.0, 0.0, 0.0))
    );
    assert_eq!(
        map.get(&key("hero"))
            .unwrap()
            .name
            .as_ref()
            .map(|n| n.as_str()),
        Some("Player")
    );
    // The hierarchy and the references inside the prefab follow the instance.
    assert_eq!(world.get::<Parent>(gun), Some(&Parent(hero)));
    assert_eq!(
        world.get::<Parent>(hero),
        Some(&Parent(map.entity(&key("world")).unwrap()))
    );
    assert_eq!(world.get::<Follow>(gun).unwrap().target, key("hero"));
    assert_eq!(
        map.get(&key("hero/gun")).unwrap().generated_by,
        Some(key("hero"))
    );
    assert_eq!(
        map.get(&key("hero")).unwrap().prefab,
        Some(path("actors/hero"))
    );
}

#[test]
fn the_prefab_name_survives_unless_the_entity_has_its_own() {
    let scene = r#"{ format: 1, entities: [ { id: "h", prefab: "@prefab:actors/hero" } ] }"#;
    let (_, map) = load(scene, &mut library()).unwrap();
    assert_eq!(
        map.get(&key("h"))
            .unwrap()
            .name
            .as_ref()
            .map(|n| n.as_str()),
        Some("Body")
    );
}

#[test]
fn a_listed_component_replaces_the_prefabs_whole() {
    let scene = r#"{ format: 1, entities: [
        { id: "h", prefab: "@prefab:actors/hero",
          components: { rigid_body: { motion: "static", mass: 1 } } } ] }"#;
    let (world, map) = load(scene, &mut library()).unwrap();
    let body = world
        .get::<RigidBody>(map.entity(&key("h")).unwrap())
        .unwrap();
    assert_eq!(
        *body,
        RigidBody {
            motion: Motion::Static,
            mass: 1.0
        }
    );
    // The component it did not list is the prefab's.
    assert_eq!(
        transform(&world, &map, "h"),
        Motor3::translation(Vec3::new(0.0, 1.0, 0.0))
    );
}

#[test]
fn two_instances_do_not_share_ids_or_entities() {
    let scene = r#"{ format: 1, entities: [
        { id: "a", prefab: "@prefab:actors/hero" },
        { id: "b", prefab: "@prefab:actors/hero" } ] }"#;
    let (world, map) = load(scene, &mut library()).unwrap();
    assert_eq!(keys(&map), ["a", "a/gun", "b", "b/gun"]);
    let (ga, gb) = (
        map.entity(&key("a/gun")).unwrap(),
        map.entity(&key("b/gun")).unwrap(),
    );
    assert_ne!(ga, gb);
    assert_eq!(world.get::<Follow>(ga).unwrap().target, key("a"));
    assert_eq!(world.get::<Follow>(gb).unwrap().target, key("b"));
}

#[test]
fn prefabs_nest_and_ids_compose() {
    let mut lib = library();
    lib.insert(
        path("squads/pair"),
        r#"{ format: 1, entities: [
            { id: "root" },
            { id: "leader", parent: "@entity:root", prefab: "@prefab:actors/hero" } ] }"#,
    );
    let scene = r#"{ format: 1, entities: [ { id: "squad", prefab: "@prefab:squads/pair" } ] }"#;
    let (world, map) = load(scene, &mut lib).unwrap();
    assert_eq!(keys(&map), ["squad", "squad/leader", "squad/leader/gun"]);
    let gun = map.entity(&key("squad/leader/gun")).unwrap();
    assert_eq!(
        world.get::<Follow>(gun).unwrap().target,
        key("squad/leader")
    );
    let leader = map.entity(&key("squad/leader")).unwrap();
    assert_eq!(
        world.get::<Parent>(leader),
        Some(&Parent(map.entity(&key("squad")).unwrap()))
    );
}

#[test]
fn a_prefab_may_hold_a_reference_to_its_own_root() {
    let mut lib = PrefabLibrary::new();
    lib.insert(
        path("p"),
        r#"{ format: 1, entities: [
            { id: "r" }, { id: "c", parent: "@entity:r", components: { follow: { target: "@entity:r" } } } ] }"#,
    );
    let (world, map) = load(
        r#"{ format: 1, entities: [ { id: "x", prefab: "@prefab:p" } ] }"#,
        &mut lib,
    )
    .unwrap();
    assert_eq!(
        world
            .get::<Follow>(map.entity(&key("x/c")).unwrap())
            .unwrap()
            .target,
        key("x")
    );
}

#[test]
fn the_same_prefab_twice_in_a_row_is_not_a_cycle() {
    let mut lib = library();
    lib.insert(
        path("twins"),
        r#"{ format: 1, entities: [
            { id: "r" },
            { id: "x", parent: "@entity:r", prefab: "@prefab:actors/hero" },
            { id: "y", parent: "@entity:r", prefab: "@prefab:actors/hero" } ] }"#,
    );
    let (_, map) = load(
        r#"{ format: 1, entities: [ { id: "t", prefab: "@prefab:twins" } ] }"#,
        &mut lib,
    )
    .unwrap();
    assert_eq!(keys(&map), ["t", "t/x", "t/x/gun", "t/y", "t/y/gun"]);
}
