//! Scene, prefab and save files by path: the sdk's front door to the data layer.

use std::fs;
use std::path::PathBuf;

use meridian_sdk::persistence::{
    Component, ComponentName, ComponentRegistry, EntityKey, NoResolver, PrefabInstance, PrefabPath,
    SaveHeader, SceneForm, SceneView, Transform, TransformCodec, World, WorldFiles,
    WorldFilesError,
};
use substrate_data::Schema;

#[derive(Debug, Clone, PartialEq, Schema)]
struct Health {
    points: u32,
}
impl Component for Health {}

#[derive(Debug, PartialEq, Schema)]
struct Progress {
    level: String,
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("meridian-sdk-wf-{name}-{}", std::process::id()));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn files(dir: &PathBuf) -> WorldFiles {
    let mut registry = ComponentRegistry::new();
    registry
        .register_with::<Transform, TransformCodec>(
            ComponentName::new("transform").expect("test setup"),
        )
        .expect("test setup");
    registry
        .register::<Health>(ComponentName::new("health").expect("test setup"))
        .expect("test setup");
    WorldFiles::in_directory(dir, registry)
}

const HERO: &str = r#"{ format: 1, entities: [
    { id: "body", components: { health: { points: 80 } } },
    { id: "gun", parent: "@entity:body", components: { health: { points: 5 } } },
] }"#;

fn scene_using(prefab: &str) -> String {
    format!(
        r#"{{ format: 1, entities: [
            {{ id: "hero", prefab: "@prefab:{prefab}", components: {{ health: {{ points: 99 }} }} }},
        ] }}"#
    )
}

fn key(text: &str) -> EntityKey {
    EntityKey::new(text).expect("test setup")
}

#[test]
fn a_scene_loads_with_its_prefab_read_from_the_directory() {
    let dir = scratch("scene");
    fs::create_dir_all(dir.join("actors")).expect("test setup");
    fs::write(dir.join("actors/hero.mprefab"), HERO).expect("test setup");
    fs::write(dir.join("level.mscene"), scene_using("actors/hero")).expect("test setup");
    let mut world = World::new();

    let scene = files(&dir)
        .load_scene("level.mscene", &mut world, &mut NoResolver)
        .expect("loads");

    let hero = scene.entity(&key("hero")).expect("test setup");
    let gun = scene.entity(&key("hero/gun")).expect("test setup");
    assert_eq!(world.get::<Health>(hero), Some(&Health { points: 99 }));
    assert_eq!(world.get::<Health>(gun), Some(&Health { points: 5 }));
    fs::remove_dir_all(&dir).expect("test setup");
}

#[test]
fn a_missing_prefab_is_reported_and_nothing_is_spawned() {
    let dir = scratch("missing");
    fs::write(dir.join("level.mscene"), scene_using("actors/nobody")).expect("test setup");
    let mut world = World::new();

    let error = files(&dir)
        .load_scene("level.mscene", &mut world, &mut NoResolver)
        .expect_err("prefab is missing");

    assert!(matches!(error, WorldFilesError::Scene(_)), "{error:?}");
    assert!(error.to_string().contains("no such prefab"), "{error}");
    assert_eq!(world.entities().count(), 0);
    fs::remove_dir_all(&dir).expect("test setup");
}

#[test]
fn a_malformed_prefab_is_an_error_not_a_panic() {
    let dir = scratch("malformed");
    fs::write(
        dir.join("broken.mprefab"),
        "{ format: 1, entities: [ { id: ",
    )
    .expect("test setup");
    fs::write(dir.join("level.mscene"), scene_using("broken")).expect("test setup");
    let mut world = World::new();

    let error = files(&dir)
        .load_scene("level.mscene", &mut world, &mut NoResolver)
        .expect_err("prefab is damaged");

    assert!(matches!(error, WorldFilesError::Scene(_)), "{error:?}");
    assert_eq!(world.entities().count(), 0);
    fs::remove_dir_all(&dir).expect("test setup");
}

#[test]
fn a_missing_scene_file_is_a_read_error() {
    let dir = scratch("nofile");
    let mut world = World::new();
    let error = files(&dir)
        .load_scene("absent.mscene", &mut world, &mut NoResolver)
        .expect_err("no file");
    assert!(matches!(error, WorldFilesError::Read(_)), "{error:?}");
    fs::remove_dir_all(&dir).expect("test setup");
}

#[test]
fn a_spawned_prefab_is_instanced_from_code() {
    let dir = scratch("spawn");
    fs::write(dir.join("hero.mprefab"), HERO).expect("test setup");
    let mut world = World::new();
    let instance = PrefabInstance::new(key("player"), PrefabPath::new("hero").expect("test setup"));

    let scene = files(&dir)
        .spawn_prefab(&instance, &mut world, &mut NoResolver)
        .expect("spawns");

    assert!(scene.entity(&key("player/gun")).is_some());
    fs::remove_dir_all(&dir).expect("test setup");
}

#[test]
fn a_scene_saves_in_both_forms_and_loads_back() {
    let dir = scratch("roundtrip");
    fs::write(dir.join("hero.mprefab"), HERO).expect("test setup");
    fs::write(dir.join("level.mscene"), scene_using("hero")).expect("test setup");
    let wf = files(&dir);
    let mut world = World::new();
    let scene = wf
        .load_scene("level.mscene", &mut world, &mut NoResolver)
        .expect("test setup");

    for (name, form) in [
        ("a.mscene", SceneForm::Text),
        ("b.mscene", SceneForm::Binary),
    ] {
        wf.save_scene(&format!("out/{name}"), SceneView::new(&world, &scene), form)
            .expect("saves");
        let mut again = World::new();
        let loaded = wf
            .load_scene(&format!("out/{name}"), &mut again, &mut NoResolver)
            .expect("reloads");
        let hero = loaded.entity(&key("hero")).expect("test setup");
        assert_eq!(again.get::<Health>(hero), Some(&Health { points: 99 }));
        assert!(loaded.entity(&key("hero/gun")).is_some());
    }
    fs::remove_dir_all(&dir).expect("test setup");
}

#[test]
fn a_save_round_trips_with_game_state_and_a_cheap_header() {
    let dir = scratch("save");
    fs::write(dir.join("hero.mprefab"), HERO).expect("test setup");
    fs::write(dir.join("level.mscene"), scene_using("hero")).expect("test setup");
    let wf = files(&dir);
    let mut world = World::new();
    let scene = wf
        .load_scene("level.mscene", &mut world, &mut NoResolver)
        .expect("test setup");
    let header = SaveHeader::new("test-game", 1_700_000_000);
    let state = Progress {
        level: "caves".to_owned(),
    };

    wf.save_game(
        "slots/one.msave",
        &header,
        SceneView::new(&world, &scene),
        &state,
    )
    .expect("saves");

    assert_eq!(
        wf.read_save_header("slots/one.msave").expect("test setup"),
        header
    );
    let mut restored = World::new();
    let loaded = wf
        .load_game::<Progress>("slots/one.msave", &mut restored, &mut NoResolver)
        .expect("loads");
    assert_eq!(loaded.state, state);
    assert!(loaded.scene.entity(&key("hero/gun")).is_some());
    fs::remove_dir_all(&dir).expect("test setup");
}

#[test]
fn a_damaged_save_is_a_typed_error() {
    let dir = scratch("damaged");
    fs::write(dir.join("bad.msave"), b"SDAT\x02garbage").expect("test setup");
    let mut world = World::new();
    let error = files(&dir)
        .load_game::<Progress>("bad.msave", &mut world, &mut NoResolver)
        .expect_err("damaged");
    assert!(matches!(error, WorldFilesError::Save(_)), "{error:?}");
    fs::remove_dir_all(&dir).expect("test setup");
}

#[test]
fn saving_outside_the_root_is_refused() {
    let dir = scratch("escape");
    let wf = files(&dir);
    let world = World::new();
    let scene = meridian_sdk::persistence::SceneMap::new(None);
    for path in ["../evil.mscene", "/abs.mscene", ""] {
        let error = wf
            .save_scene(path, SceneView::new(&world, &scene), SceneForm::Text)
            .expect_err("refused");
        assert!(
            matches!(error, WorldFilesError::PathRejected { .. }),
            "{path}"
        );
    }
    fs::remove_dir_all(&dir).expect("test setup");
}
