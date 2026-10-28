//! `.msave`: the envelope, the scene inside it, and the game's own state.

mod common;

use common::{Motion, RigidBody, key, registry, transform};
use meridian_data_substrate::{
    LoadedSave, NoResolver, Parent, PrefabLibrary, PrefabPath, SAVE_FORMAT, SaveError, SaveHeader,
    SceneEntity, SceneMap,
};
use meridian_ecs::{Transform, World};
use meridian_gac::{Motor3, Vec3};
use substrate_data::{Schema, Stream, binary, text};

/// What a game keeps: its own version first (the evolution rule for
/// state), a few numbers, and a recording kept as a stream.
#[derive(Debug, Schema)]
struct Progress {
    version: u32,
    level: String,
    score: i64,
    #[data(default)]
    lives: u32,
    recording: Stream<u32>,
}

fn progress() -> Progress {
    Progress {
        version: 1,
        level: "caves".to_owned(),
        score: 4200,
        lives: 3,
        recording: (0..50_000u32).map(|i| i % 97).collect(),
    }
}

/// A world with a parent and a child, and its scene map.
fn playing() -> (World, SceneMap) {
    let mut world = World::new();
    let mut scene = SceneMap::new(None);
    let hero = world.spawn();
    world.insert(
        hero,
        Transform {
            motor: Motor3::translation(Vec3::new(1.0, 2.0, 3.0)),
        },
    );
    world.insert(
        hero,
        RigidBody {
            motion: Motion::Dynamic,
            mass: 70.0,
        },
    );
    scene.push(SceneEntity::new(key("hero"), hero)).unwrap();
    let pet = world.spawn();
    world.insert(pet, Parent(hero));
    scene.push(SceneEntity::new(key("pet"), pet)).unwrap();
    (world, scene)
}

fn saved() -> Vec<u8> {
    let (world, scene) = playing();
    registry().save_game(
        &SaveHeader::new("demo", 1_700_000_000),
        &world,
        &scene,
        &progress(),
    )
}

fn load(bytes: &[u8]) -> Result<(World, LoadedSave<Progress>), SaveError> {
    let mut world = World::new();
    let loaded = registry().load_game(
        bytes,
        &mut world,
        &mut NoResolver,
        &mut PrefabLibrary::new(),
    )?;
    Ok((world, loaded))
}

#[test]
fn a_save_round_trips_scene_and_state() {
    let bytes = saved();
    assert!(bytes.starts_with(b"SDAT"));
    let (world, loaded) = load(&bytes).unwrap();
    assert_eq!(loaded.header, SaveHeader::new("demo", 1_700_000_000));
    assert_eq!(loaded.header.format, SAVE_FORMAT);
    let state = &loaded.state;
    assert_eq!(
        (
            state.version,
            state.level.as_str(),
            state.score,
            state.lives
        ),
        (1, "caves", 4200, 3)
    );
    assert_eq!(state.recording.len(), 50_000);
    assert_eq!(state.recording.iter().nth(98), Some(Ok(1)));

    let keys: Vec<&str> = loaded
        .scene
        .entities()
        .iter()
        .map(|e| e.key.as_str())
        .collect();
    assert_eq!(keys, ["hero", "pet"]);
    let hero = loaded.scene.entity(&key("hero")).unwrap();
    assert_eq!(
        transform(&world, &loaded.scene, "hero"),
        Motor3::translation(Vec3::new(1.0, 2.0, 3.0))
    );
    assert_eq!(world.get::<RigidBody>(hero).unwrap().mass, 70.0);
    let pet = loaded.scene.entity(&key("pet")).unwrap();
    assert_eq!(world.get::<Parent>(pet), Some(&Parent(hero)));
}

#[test]
fn the_scene_is_a_full_scene_document_and_no_generation_is_written() {
    let (mut world, scene) = playing();
    // Bump a generation: spawn and despawn a few times.
    for _ in 0..3 {
        let spare = world.spawn();
        world.despawn(spare);
    }
    let reg = registry();
    let bytes = reg.save_game(&SaveHeader::new("demo", 5), &world, &scene, &progress());
    let root = binary::decode(&bytes).unwrap();
    let embedded = root.get("scene").unwrap();
    assert_eq!(embedded, &reg.scene_document(&world, &scene));
    assert!(!text::to_string_pretty(&root).contains("generation"));
}

#[test]
fn the_header_and_scene_read_without_the_state() {
    let mut bytes = saved();
    // Damage the end of the file, inside the state's recording.
    let last = bytes.len() - 1;
    bytes[last] ^= 0xff;
    bytes[last - 1] ^= 0xff;
    bytes[last - 2] = 0xee;
    assert_eq!(
        SaveHeader::read(&bytes).unwrap(),
        SaveHeader::new("demo", 1_700_000_000)
    );
    let mut world = World::new();
    let (header, scene) = registry()
        .load_game_scene(
            &bytes,
            &mut world,
            &mut NoResolver,
            &mut PrefabLibrary::new(),
        )
        .unwrap();
    assert_eq!(header.game, "demo");
    assert_eq!(scene.entities().len(), 2);
    // The recording reads lazily (substrate-data 0.4): loading succeeds and
    // the damage shows when the recording is walked.
    let mut world = World::new();
    let loaded = registry()
        .load_game::<Progress>(
            &bytes,
            &mut world,
            &mut NoResolver,
            &mut PrefabLibrary::new(),
        )
        .expect("the damage is inside the lazily read recording");
    assert!(loaded.state.recording.iter().any(|item| item.is_err()));
}

#[test]
fn an_envelope_in_the_canonical_form_reads_too() {
    // Another writer (or `binary::upgrade`) may order and shape the bytes
    // differently: the state is a plain list there, a stream's items.
    let original = saved();
    let rewritten = binary::encode(&binary::decode(&original).unwrap());
    let (_, loaded) = load(&rewritten).unwrap();
    assert_eq!(loaded.state.recording.len(), 50_000);
    assert_eq!(loaded.scene.entities().len(), 2);
}

#[test]
fn prefab_instances_are_remade_on_load_not_stored() {
    let mut library = PrefabLibrary::new();
    library.insert(
        PrefabPath::new("actors/hero").unwrap(),
        r#"{ format: 1, entities: [ { id: "body" }, { id: "gun", parent: "@entity:body" } ] }"#,
    );
    let reg = registry();
    let mut world = World::new();
    let scene = reg
        .load_scene_with_prefabs(
            br#"{ format: 1, entities: [ { id: "hero", prefab: "@prefab:actors/hero" } ] }"#,
            &mut world,
            &mut NoResolver,
            &mut library,
        )
        .unwrap();
    assert_eq!(scene.entities().len(), 2);
    let bytes = reg.save_game(&SaveHeader::new("demo", 1), &world, &scene, &());
    let embedded = binary::decode(&bytes).unwrap();
    assert_eq!(
        embedded
            .get("scene")
            .unwrap()
            .get("entities")
            .and_then(|e| e.as_list())
            .map(<[_]>::len),
        Some(1)
    );
    let mut again = World::new();
    let loaded: LoadedSave<()> = reg
        .load_game(&bytes, &mut again, &mut NoResolver, &mut library)
        .unwrap();
    let keys: Vec<&str> = loaded
        .scene
        .entities()
        .iter()
        .map(|e| e.key.as_str())
        .collect();
    assert_eq!(keys, ["hero", "hero/gun"]);
    // Without the prefab source the save names the prefab it cannot find.
    let mut empty = World::new();
    let err = reg
        .load_game::<()>(
            &bytes,
            &mut empty,
            &mut NoResolver,
            &mut PrefabLibrary::new(),
        )
        .unwrap_err();
    assert!(matches!(err, SaveError::Scene(_)), "{err}");
}
