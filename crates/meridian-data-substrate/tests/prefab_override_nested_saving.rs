//! Saving prefab instances, continued: added and removed components,
//! nested instances, game saves, and a prefab that can't be read.

mod common;
mod saving_support;

use common::{Follow, key, registry, transform};
use meridian_data_substrate::{LoadedSave, NoResolver, SaveHeader, SceneSaveError, SceneView};
use meridian_ecs::{Transform, World};
use meridian_gac::{Motor3, Vec3};
use saving_support::{SCENE, at, library, load, path, save, state, value_at};
use substrate_data::Value;

#[test]
fn added_removed_and_unpatchable_components_are_written_whole_or_removed() {
    let (mut world, map) = load(SCENE.as_bytes());
    // Added to the instance, removed from the child.
    world.insert(
        at(&map, "h"),
        Follow {
            target: key("other"),
        },
    );
    world.remove::<Follow>(at(&map, "h/gun"));
    let saved = save(&world, &map);
    let added = value_at(&saved, "entities[0].overrides.h.components.follow");
    assert!(added.is_some(), "{saved}");
    let removed = value_at(&saved, "entities[0].overrides.h/gun.remove[0]");
    assert_eq!(removed, Some(Value::Str("follow".to_owned())), "{saved}");
    let (again, back) = load(saved.as_bytes());
    assert_eq!(state(&again, &back), state(&world, &map));
    assert!(again.get::<Follow>(at(&back, "h/gun")).is_none());
    assert_eq!(
        again.get::<Follow>(at(&back, "h")).unwrap().target,
        key("other")
    );
}

#[test]
fn nested_instances_save_overrides_under_the_outer_instance() {
    let mut lib = library();
    lib.insert(
        path("squads/pair"),
        r#"{ format: 1, entities: [
            { id: "root" },
            { id: "leader", parent: "@entity:root", prefab: "@prefab:actors/hero" } ] }"#,
    );
    let scene = br#"{ format: 1, entities: [ { id: "s", prefab: "@prefab:squads/pair" } ] }"#;
    let mut world = World::new();
    let map = registry()
        .load_scene_with_prefabs(scene, &mut world, &mut NoResolver, &mut lib)
        .unwrap();
    world.insert(at(&map, "s/leader/gun"), Follow { target: key("s") });
    let saved = registry()
        .save_scene_with_prefabs(SceneView::new(&world, &map), &mut lib)
        .unwrap();
    let target = value_at(
        &saved,
        "entities[0].overrides.s/leader/gun.fields.follow.target",
    );
    assert!(target.is_some(), "{saved}");
    assert!(
        value_at(&saved, "entities[0].overrides.s/leader").is_none(),
        "{saved}"
    );
    let mut again = World::new();
    let back = registry()
        .load_scene_with_prefabs(saved.as_bytes(), &mut again, &mut NoResolver, &mut lib)
        .unwrap();
    assert_eq!(state(&again, &back), state(&world, &map));
}

#[test]
fn a_game_save_keeps_edits_to_generated_entities() {
    let (mut world, map) = load(SCENE.as_bytes());
    world.insert(
        at(&map, "h/gun"),
        Transform {
            motor: Motor3::translation(Vec3::new(0.0, 0.0, 8.0)),
        },
    );
    let reg = registry();
    let header = SaveHeader::new("demo", 1);
    let bytes = reg
        .save_game_with_prefabs(&header, SceneView::new(&world, &map), &7u32, &mut library())
        .unwrap();
    let mut loaded_world = World::new();
    let loaded: LoadedSave<u32> = reg
        .load_game(&bytes, &mut loaded_world, &mut NoResolver, &mut library())
        .unwrap();
    assert_eq!(loaded.state, 7);
    assert_eq!(
        transform(&loaded_world, &loaded.scene, "h/gun"),
        Motor3::translation(Vec3::new(0.0, 0.0, 8.0))
    );
}

#[test]
fn saving_without_the_prefab_names_the_instance() {
    let (world, map) = load(SCENE.as_bytes());
    let err = registry()
        .save_scene_with_prefabs(
            SceneView::new(&world, &map),
            &mut meridian_data_substrate::PrefabLibrary::new(),
        )
        .unwrap_err();
    let SceneSaveError::Prefab {
        instance, prefab, ..
    } = &err
    else {
        panic!("{err}");
    };
    assert_eq!((instance, prefab), (&key("h"), &path("actors/hero")));
    assert!(err.to_string().contains("no such prefab"), "{err}");
}
