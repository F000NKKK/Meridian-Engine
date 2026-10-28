//! `.msave` problems: a format that is too new, bytes that are damaged,
//! fields that are missing, and a scene that doesn't load.

mod common;

use common::{key, registry, transform};
use meridian_data_substrate::{
    NoResolver, PrefabLibrary, SAVE_FORMAT, SaveError, SaveHeader, SceneEntity, SceneMap,
};
use meridian_ecs::{Transform, World};
use meridian_gac::{Motor3, Vec3};
use substrate_data::{Map, Value, binary, text};

fn saved() -> Vec<u8> {
    let mut world = World::new();
    let mut scene = SceneMap::new(None);
    let a = world.spawn();
    world.insert(
        a,
        Transform {
            motor: Motor3::translation(Vec3::X),
        },
    );
    scene.push(SceneEntity::new(key("a"), a)).unwrap();
    registry().save_game(&SaveHeader::new("demo", 1_700_000_000), &world, &scene, &())
}

fn load(bytes: &[u8]) -> Result<(), SaveError> {
    let mut world = World::new();
    let loaded = registry().load_game::<()>(
        bytes,
        &mut world,
        &mut NoResolver,
        &mut PrefabLibrary::new(),
    );
    loaded.map(drop)
}

#[test]
fn a_good_save_loads() {
    let mut world = World::new();
    let loaded = registry()
        .load_game::<()>(
            &saved(),
            &mut world,
            &mut NoResolver,
            &mut PrefabLibrary::new(),
        )
        .unwrap();
    assert_eq!(
        transform(&world, &loaded.scene, "a"),
        Motor3::translation(Vec3::X)
    );
    assert_eq!(loaded.header.created, 1_700_000_000);
}

/// An envelope with exactly these fields, in this order.
fn envelope(fields: &[(&str, Value)]) -> Vec<u8> {
    let mut map = Map::new();
    for (name, value) in fields {
        map.insert(*name, value.clone());
    }
    binary::encode(&Value::Map(map))
}

fn scene_doc() -> Value {
    text::parse("{ format: 1, entities: [ { id: \"a\" } ] }")
        .unwrap()
        .value
}

#[test]
fn a_newer_envelope_format_is_refused_before_anything_else() {
    let newer = envelope(&[
        ("format", Value::Int(i64::from(SAVE_FORMAT) + 1)),
        (
            "anything",
            Value::Str("a field this version never heard of".into()),
        ),
    ]);
    let expected = SaveError::UnsupportedFormat {
        found: SAVE_FORMAT + 1,
        supported: SAVE_FORMAT,
    };
    assert_eq!(SaveHeader::read(&newer).unwrap_err(), expected);
    assert_eq!(load(&newer).unwrap_err(), expected);
    let mut world = World::new();
    let err = registry()
        .load_game_scene(
            &newer,
            &mut world,
            &mut NoResolver,
            &mut PrefabLibrary::new(),
        )
        .unwrap_err();
    assert_eq!(err, expected);
    assert!(expected.to_string().contains("unsupported save format 2"));
}

#[test]
fn malformed_saves_are_typed_errors() {
    let reg = registry();
    let bytes = saved();
    for cut in (0..bytes.len()).step_by(7).chain([bytes.len() - 1]) {
        let part = &bytes[..cut];
        assert!(load(part).is_err(), "{cut}");
        // Reading a part of a save is an error or reads what is there; never a panic.
        drop(SaveHeader::read(part));
        let mut world = World::new();
        let r = reg.load_game_scene(part, &mut world, &mut NoResolver, &mut PrefabLibrary::new());
        if r.is_err() {
            assert_eq!(world.entities().count(), 0, "{cut}");
        }
    }
    assert!(matches!(
        load(b"NOPE").unwrap_err(),
        SaveError::Malformed { offset: 0, .. }
    ));
    let list = binary::encode(&Value::List(vec![]));
    assert_eq!(load(&list).unwrap_err(), SaveError::NotAnEnvelope);
    assert_eq!(
        SaveHeader::read(&list).unwrap_err(),
        SaveError::NotAnEnvelope
    );
}

#[test]
fn missing_and_mistyped_fields_are_named() {
    let ok = |extra: Vec<(&str, Value)>| {
        let mut fields = vec![
            ("format", Value::Int(1)),
            ("game", Value::Str("g".into())),
            ("created", Value::Int(5)),
        ];
        fields.extend(extra);
        envelope(&fields)
    };
    // No scene: the header reads, the scene does not.
    let no_scene = ok(vec![]);
    assert_eq!(SaveHeader::read(&no_scene).unwrap().created, 5);
    let mut world = World::new();
    let r = registry().load_game_scene(
        &no_scene,
        &mut world,
        &mut NoResolver,
        &mut PrefabLibrary::new(),
    );
    assert_eq!(r.unwrap_err(), SaveError::MissingField { field: "scene" });
    // No state.
    let no_state = ok(vec![("scene", scene_doc())]);
    assert_eq!(
        load(&no_state).unwrap_err(),
        SaveError::MissingField { field: "state" }
    );
    // No game, a game that is not text, no format.
    let bytes = envelope(&[("format", Value::Int(1)), ("created", Value::Int(5))]);
    assert_eq!(
        SaveHeader::read(&bytes).unwrap_err(),
        SaveError::MissingField { field: "game" }
    );
    let bytes = envelope(&[
        ("format", Value::Int(1)),
        ("game", Value::Int(1)),
        ("created", Value::Int(5)),
    ]);
    assert!(matches!(
        SaveHeader::read(&bytes).unwrap_err(),
        SaveError::BadField { field: "game", .. }
    ));
    let bytes = envelope(&[("game", Value::Str("g".into()))]);
    assert_eq!(
        SaveHeader::read(&bytes).unwrap_err(),
        SaveError::MissingField { field: "format" }
    );
    let bytes = envelope(&[("format", Value::Str("one".into()))]);
    assert!(matches!(
        SaveHeader::read(&bytes).unwrap_err(),
        SaveError::BadField {
            field: "format",
            ..
        }
    ));
}

#[test]
fn problems_in_the_scene_come_with_their_paths() {
    let doc =
        text::parse("{ format: 1, entities: [ { id: \"a\", components: { trasform: {} } } ] }")
            .unwrap()
            .value;
    let bytes = envelope(&[
        ("format", Value::Int(1)),
        ("game", Value::Str("g".into())),
        ("created", Value::Int(0)),
        ("scene", doc),
        ("state", Value::Null),
    ]);
    let mut world = World::new();
    let err = registry()
        .load_game::<()>(
            &bytes,
            &mut world,
            &mut NoResolver,
            &mut PrefabLibrary::new(),
        )
        .unwrap_err();
    let SaveError::Scene(load) = &err else {
        panic!("{err}")
    };
    assert_eq!(
        load.issues()[0].path.to_string(),
        "entities[0].components.trasform"
    );
    assert_eq!(world.entities().count(), 0);
}
