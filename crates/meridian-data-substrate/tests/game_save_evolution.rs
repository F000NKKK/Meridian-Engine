//! `.msave` state evolution: a newer game's state fails loudly, an older
//! game's reads (Substrate Data's evolution rules, `docs/evolution.md`).

mod common;

use common::{key, registry, transform};
use meridian_data_substrate::{
    LoadedSave, NoResolver, PrefabLibrary, SaveError, SaveHeader, SceneEntity, SceneMap,
};
use meridian_ecs::{Transform, World};
use meridian_gac::{Motor3, Vec3};
use substrate_data::{Problem, Schema, Stream};

/// The game as it is now.
#[derive(Debug, Schema)]
struct Progress {
    version: u32,
    level: String,
    score: i64,
    #[data(default)]
    lives: u32,
    recording: Stream<u32>,
}

/// The same game one version later: a field appended with a default.
#[derive(Debug, Schema)]
struct ProgressV2 {
    version: u32,
    level: String,
    score: i64,
    #[data(default)]
    lives: u32,
    recording: Stream<u32>,
    #[data(default)]
    coins: u32,
}

/// The game one version earlier than `Progress`: it has no `lives`.
#[derive(Debug, Schema)]
struct ProgressV0 {
    version: u32,
    level: String,
    score: i64,
    recording: Stream<u32>,
}

/// A world with one entity under the key `p`.
fn one_entity() -> (World, SceneMap) {
    let mut world = World::new();
    let mut scene = SceneMap::new(None);
    let p = world.spawn();
    world.insert(p, Transform::from_motor(Motor3::translation(Vec3::Y)));
    scene.push(SceneEntity::new(key("p"), p)).unwrap();
    (world, scene)
}

fn load(bytes: &[u8]) -> Result<LoadedSave<Progress>, SaveError> {
    let mut world = World::new();
    registry().load_game(
        bytes,
        &mut world,
        &mut NoResolver,
        &mut PrefabLibrary::new(),
    )
}

#[test]
fn a_newer_games_state_fails_loudly_and_an_older_one_reads() {
    let (world, scene) = one_entity();
    let reg = registry();
    let header = SaveHeader::new("demo", 1);
    // A newer game appended `coins`; the older game reads it as unknown.
    let newer = ProgressV2 {
        version: 2,
        level: "sky".into(),
        score: 1,
        lives: 1,
        recording: Stream::from(vec![1, 2]),
        coins: 9,
    };
    let bytes = reg.save_game(&header, &world, &scene, &newer);
    let err = load(&bytes).unwrap_err();
    let SaveError::State(problems) = &err else {
        panic!("{err}")
    };
    assert_eq!(problems[0].code, Some(Problem::UnknownField.into()));
    assert_eq!(problems[0].path.to_string(), "state.coins");
    assert!(err.to_string().contains("coins"), "{err}");
    // A save from the older game is read by the newer one: appended fields default.
    let older = reg.save_game(
        &header,
        &world,
        &scene,
        &Progress {
            version: 1,
            level: "caves".into(),
            score: 1,
            lives: 3,
            recording: Stream::from(vec![7]),
        },
    );
    let mut w = World::new();
    let read: LoadedSave<ProgressV2> = reg
        .load_game(&older, &mut w, &mut NoResolver, &mut PrefabLibrary::new())
        .unwrap();
    assert_eq!((read.state.coins, read.state.lives), (0, 3));
    assert_eq!(
        transform(&w, &read.scene, "p"),
        Motor3::translation(Vec3::Y)
    );
    // And a field the older game lacks (`lives`) is the same unknown field.
    let old = ProgressV0 {
        version: 0,
        level: "x".into(),
        score: 0,
        recording: Stream::from(vec![]),
    };
    let bytes = reg.save_game(&header, &world, &scene, &old);
    assert!(
        load(&bytes).is_ok(),
        "`lives` has a default, so the older save reads"
    );
}
