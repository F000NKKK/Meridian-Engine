//! `.mprefab` from code and on disk: instancing at run time, loading a prefab
//! as itself, saving, and both forms.

mod common;

use common::{key, registry, transform};
use meridian_data_substrate::{
    LoadError, NoResolver, PrefabInstance, PrefabLibrary, PrefabPath, SceneMap,
};
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
fn instantiating_from_code_names_the_instance() {
    let mut world = World::new();
    let map = registry()
        .instantiate_prefab(
            &PrefabInstance::new(key("npc_1"), path("actors/hero")),
            &mut world,
            &mut NoResolver,
            &mut library(),
        )
        .unwrap();
    assert_eq!(keys(&map), ["npc_1", "npc_1/gun"]);
    assert_eq!(world.entities().count(), 2);
    let mut world = World::new();
    let err = registry()
        .instantiate_prefab(
            &PrefabInstance::new(key("n"), path("nope")),
            &mut world,
            &mut NoResolver,
            &mut library(),
        )
        .unwrap_err();
    assert_eq!(err.issues()[0].path.to_string(), "entities[0].prefab");
    assert_eq!(world.entities().count(), 0);
}

#[test]
fn a_prefab_loads_as_itself_and_saves_in_both_forms() {
    let reg = registry();
    let mut world = World::new();
    let map = reg
        .load_prefab(HERO.as_bytes(), &mut world, &mut NoResolver, &mut library())
        .unwrap();
    assert_eq!(keys(&map), ["body", "gun"]);
    assert_eq!(
        transform(&world, &map, "body"),
        Motor3::translation(Vec3::new(0.0, 1.0, 0.0))
    );
    assert!(map.name().is_none());
    let text = reg.save_prefab(&world, &map);
    assert!(
        substrate_data::text::parse(&text)
            .unwrap()
            .value
            .get("name")
            .is_none(),
        "{text}"
    );
    let bytes = reg.save_prefab_binary(&world, &map);
    for source in [text.as_bytes(), bytes.as_slice()] {
        let mut again = World::new();
        let back = reg
            .load_prefab(source, &mut again, &mut NoResolver, &mut library())
            .unwrap();
        assert_eq!(reg.save_prefab(&again, &back), text);
    }
}

#[test]
fn saving_an_instanced_scene_leaves_generated_entities_to_the_prefab() {
    let reg = registry();
    let scene = r#"{ format: 1, entities: [
        { id: "hero", prefab: "@prefab:actors/hero",
          components: { rigid_body: { motion: "static", mass: 2 } } } ] }"#;
    let (world, map) = load(scene, &mut library()).unwrap();
    let saved = reg.save_scene(&world, &map);
    assert!(!saved.contains("hero/gun"), "{saved}");
    assert!(saved.contains("@prefab:actors/hero"), "{saved}");
    // Loading what was saved gives the same world shape.
    let (world2, map2) = load(&saved, &mut library()).unwrap();
    assert_eq!(keys(&map2), ["hero", "hero/gun"]);
    assert_eq!(reg.save_scene(&world2, &map2), saved);
    // The binary form too.
    let mut world3 = World::new();
    let map3 = reg
        .load_scene_with_prefabs(
            &reg.save_scene_binary(&world, &map),
            &mut world3,
            &mut NoResolver,
            &mut library(),
        )
        .unwrap();
    assert_eq!(keys(&map3), ["hero", "hero/gun"]);
}

#[test]
fn a_prefab_in_binary_is_instanced_like_text() {
    let reg = registry();
    let mut world = World::new();
    let map = reg
        .load_prefab(
            HERO.as_bytes(),
            &mut world,
            &mut NoResolver,
            &mut PrefabLibrary::new(),
        )
        .unwrap();
    let mut lib = PrefabLibrary::new();
    lib.insert(path("actors/hero"), reg.save_prefab_binary(&world, &map));
    let (_, map) = load(
        r#"{ format: 1, entities: [ { id: "h", prefab: "@prefab:actors/hero" } ] }"#,
        &mut lib,
    )
    .unwrap();
    assert_eq!(keys(&map), ["h", "h/gun"]);
}
