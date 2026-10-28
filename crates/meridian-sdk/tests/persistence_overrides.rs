//! Saving prefab instances by path: the edits to the entities a prefab made
//! are kept as overrides, and a prefab that is gone is a typed error.

use std::fs;
use std::path::PathBuf;

use meridian_sdk::persistence::{
    Component, ComponentName, ComponentRegistry, EntityKey, NoResolver, SceneForm, SceneView,
    World, WorldFiles, WorldFilesError,
};
use substrate_data::Schema;

#[derive(Debug, Clone, PartialEq, Schema)]
struct Health {
    points: u32,
}
impl Component for Health {}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("meridian-sdk-wo-{name}-{}", std::process::id()));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn files(dir: &PathBuf) -> WorldFiles {
    let mut registry = ComponentRegistry::new();
    registry
        .register::<Health>(ComponentName::new("health").expect("test setup"))
        .expect("test setup");
    WorldFiles::in_directory(dir, registry)
}

const HERO: &str = r#"{ format: 1, entities: [
    { id: "body", components: { health: { points: 80 } } },
    { id: "gun", parent: "@entity:body", components: { health: { points: 5 } } },
] }"#;

const LEVEL: &str = r#"{ format: 1, entities: [
    { id: "hero", prefab: "@prefab:hero", components: { health: { points: 99 } } },
] }"#;

fn key(text: &str) -> EntityKey {
    EntityKey::new(text).expect("test setup")
}

#[test]
fn an_edit_to_a_generated_entity_survives_a_save_as_an_override() {
    let dir = scratch("override");
    fs::write(dir.join("hero.mprefab"), HERO).expect("test setup");
    fs::write(dir.join("level.mscene"), LEVEL).expect("test setup");
    let wf = files(&dir);
    let mut world = World::new();
    let scene = wf
        .load_scene("level.mscene", &mut world, &mut NoResolver)
        .expect("test setup");
    let gun = scene.entity(&key("hero/gun")).expect("test setup");
    world.insert(gun, Health { points: 1 });

    for (name, form) in [("a", SceneForm::Text), ("b", SceneForm::Binary)] {
        let path = format!("out/{name}.mscene");
        wf.save_scene(&path, SceneView::new(&world, &scene), form)
            .expect("saves");
        let mut again = World::new();
        let loaded = wf
            .load_scene(&path, &mut again, &mut NoResolver)
            .expect("reloads");
        let edited = loaded.entity(&key("hero/gun")).expect("test setup");
        assert_eq!(again.get::<Health>(edited), Some(&Health { points: 1 }));
        let hero = loaded.entity(&key("hero")).expect("test setup");
        assert_eq!(again.get::<Health>(hero), Some(&Health { points: 99 }));
    }
    let text = fs::read_to_string(dir.join("out/a.mscene")).expect("test setup");
    assert!(
        text.contains("overrides") && text.contains("hero/gun"),
        "{text}"
    );
    fs::remove_dir_all(&dir).expect("test setup");
}

#[test]
fn saving_an_instance_whose_prefab_is_gone_is_a_typed_error() {
    let dir = scratch("gone");
    fs::write(dir.join("hero.mprefab"), HERO).expect("test setup");
    fs::write(dir.join("level.mscene"), LEVEL).expect("test setup");
    let wf = files(&dir);
    let mut world = World::new();
    let scene = wf
        .load_scene("level.mscene", &mut world, &mut NoResolver)
        .expect("test setup");
    fs::remove_file(dir.join("hero.mprefab")).expect("test setup");

    let error = wf
        .save_scene(
            "out.mscene",
            SceneView::new(&world, &scene),
            SceneForm::Text,
        )
        .expect_err("the prefab is gone");

    assert!(matches!(error, WorldFilesError::Instance(_)), "{error}");
    assert!(!dir.join("out.mscene").exists());
    fs::remove_dir_all(&dir).expect("test setup");
}
