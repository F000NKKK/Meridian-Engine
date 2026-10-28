//! The binary form of `.mscene`: the same documents, the same problems.

mod common;

use common::{Follow, Motion, RigidBody, key, name, registry, transform};
use meridian_data_substrate::{
    DisplayName, NoResolver, Parent, SceneEntity, SceneMap, SceneProblem, UnknownComponents,
};
use meridian_ecs::{Transform, World};
use meridian_gac::{Motor3, Vec3};
use substrate_data::{Problem, binary, text};

const LEVEL: &str = r#"// A hand-written level.
{
  format: 1,
  name: "Level 1",
  entities: [
    { id: "world", components: { transform: { motor: { rotor: [1, 0, 0, 0] } } } },
    {
      id: "hero",
      name: "Hero",
      parent: "@entity:world",
      components: {
        transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 1.5, 0] } },
        rigid_body: { motion: "dynamic", mass: 80 },
      },
    },
    { id: "camera", parent: "@entity:hero", components: { follow: { target: "@entity:hero" } } },
  ],
}
"#;

fn loaded(text: &str) -> (World, SceneMap) {
    let mut world = World::new();
    let scene = registry()
        .load_scene(text, &mut world, &mut NoResolver)
        .unwrap();
    (world, scene)
}

#[test]
fn text_to_binary_to_text_is_the_same_document() {
    let reg = registry();
    let (world, scene) = loaded(LEVEL);
    let bytes = reg.save_scene_binary(&world, &scene);
    assert!(bytes.starts_with(b"SDAT"));

    let mut again = World::new();
    let scene2 = reg
        .load_scene_binary(&bytes, &mut again, &mut NoResolver)
        .unwrap();
    assert_eq!(
        reg.save_scene(&world, &scene),
        reg.save_scene(&again, &scene2)
    );
    assert_eq!(
        reg.scene_document(&world, &scene),
        reg.scene_document(&again, &scene2)
    );
    // Binary back to text is the text the scene saves as.
    let spelled = text::to_string_pretty(&binary::decode(&bytes).unwrap());
    assert_eq!(
        text::parse(&spelled).unwrap().value,
        text::parse(&reg.save_scene(&world, &scene)).unwrap().value
    );
    assert_eq!(scene2.name().map(DisplayName::as_str), Some("Level 1"));
    let camera = scene2.entity(&key("camera")).unwrap();
    assert_eq!(again.get::<Follow>(camera).unwrap().target, key("hero"));
    assert!(again.get::<Parent>(camera).is_some());
    assert_eq!(
        transform(&again, &scene2, "hero"),
        Motor3::translation(Vec3::new(0.0, 1.5, 0.0))
    );
}

#[test]
fn the_magic_decides_the_form() {
    let reg = registry();
    let (world, scene) = loaded(LEVEL);
    let binary = reg.save_scene_binary(&world, &scene);
    let text = reg.save_scene(&world, &scene);
    for bytes in [binary.as_slice(), text.as_bytes(), LEVEL.as_bytes()] {
        let mut w = World::new();
        let scene = reg
            .load_scene_bytes(bytes, &mut w, &mut NoResolver)
            .unwrap();
        assert_eq!(scene.entities().len(), 3);
    }
}

#[test]
fn unknown_components_pass_through_binary_too() {
    let mut reg = registry();
    reg.set_unknown_components(UnknownComponents::Keep);
    let src = r#"{ format: 1, entities: [
        { id: "a", components: { light: { color: [1, 0.5, 0], power: 3 }, rigid_body: { motion: "static", mass: 1 } } } ] }"#;
    let mut world = World::new();
    let scene = reg.load_scene(src, &mut world, &mut NoResolver).unwrap();
    let bytes = reg.save_scene_binary(&world, &scene);

    let mut w2 = World::new();
    let scene2 = reg
        .load_scene_binary(&bytes, &mut w2, &mut NoResolver)
        .unwrap();
    let kept = &scene2.entities()[0].kept;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].name.as_str(), "light");
    assert_eq!(reg.save_scene(&w2, &scene2), reg.save_scene(&world, &scene));
    assert_eq!(reg.save_scene_binary(&w2, &scene2), bytes);
}

#[test]
fn binary_problems_have_paths_and_no_lines() {
    let reg = registry();
    let doc = text::parse(
        r#"{ format: 1, entities: [
        { id: "a", components: { trasform: {} } },
        { id: "a", components: { rigid_body: { motion: "dynamic" } } } ] }"#,
    )
    .unwrap()
    .value;
    let mut world = World::new();
    let err = reg
        .load_scene_binary(&binary::encode(&doc), &mut world, &mut NoResolver)
        .unwrap_err();
    assert_eq!(world.entities().count(), 0, "all or nothing");
    let issues = err.issues();
    assert!(
        issues
            .iter()
            .all(|i| i.line.is_none() && i.offset.is_none())
    );
    let paths: Vec<String> = issues.iter().map(|i| i.path.to_string()).collect();
    assert!(
        paths.contains(&"entities[0].components.trasform".to_owned()),
        "{paths:?}"
    );
    assert!(paths.contains(&"entities[1].id".to_owned()), "{paths:?}");
    assert!(
        paths.contains(&"entities[1].components.rigid_body".to_owned()),
        "{paths:?}"
    );
    let unknown = issues
        .iter()
        .find_map(|i| match &i.problem {
            SceneProblem::UnknownComponent { suggestion, .. } => suggestion.clone(),
            _ => None,
        })
        .unwrap();
    assert_eq!(unknown, name("transform"));
}

#[test]
fn an_unsupported_format_is_typed_in_binary() {
    let doc = text::parse("{ format: 7, entities: [] }").unwrap().value;
    let mut world = World::new();
    let err = registry()
        .load_scene_binary(&binary::encode(&doc), &mut world, &mut NoResolver)
        .unwrap_err();
    assert!(matches!(
        err.issues()[0].problem,
        SceneProblem::UnsupportedFormat { found: 7 }
    ));
}

fn fails_with_offset(bytes: &[u8]) -> meridian_data_substrate::SceneIssue {
    let mut world = World::new();
    let err = registry()
        .load_scene_binary(bytes, &mut world, &mut NoResolver)
        .unwrap_err();
    assert_eq!(world.entities().count(), 0);
    assert_eq!(err.issues().len(), 1);
    let issue = err.issues()[0].clone();
    assert!(
        matches!(
            issue.problem,
            SceneProblem::Data {
                problem: Problem::BadBinary,
                ..
            }
        ),
        "{issue}"
    );
    issue
}

#[test]
fn malformed_bytes_are_typed_errors_with_offsets() {
    // Not SDAT at all, a version this does not know, nothing after the header.
    assert_eq!(fails_with_offset(b"NOPE\x02\x07\x00").offset, Some(0));
    assert!(fails_with_offset(b"SDAT\x09\x07\x00").offset.is_some());
    assert!(fails_with_offset(b"SDAT\x02").offset.is_some());
    // A map that sets `a` twice: SDAT 2, map, 2 pairs, "a" = 0, "a" again (table entry 0) = 0.
    let twice = b"SDAT\x02\x07\x02\x03a\x90\x00\x90";
    let issue = fails_with_offset(twice);
    assert!(issue.to_string().contains("already set"), "{issue}");
    assert!(issue.to_string().starts_with("byte "), "{issue}");
    // Trailing bytes after a complete value.
    let (world, scene) = loaded(LEVEL);
    let mut bytes = registry().save_scene_binary(&world, &scene);
    bytes.push(0);
    assert!(fails_with_offset(&bytes).offset.is_some());
}

#[test]
fn every_truncation_and_corruption_is_an_error_not_a_panic() {
    let reg = registry();
    let (world, scene) = loaded(LEVEL);
    let bytes = reg.save_scene_binary(&world, &scene);
    for cut in 0..bytes.len() {
        let mut w = World::new();
        let r = reg.load_scene_binary(&bytes[..cut], &mut w, &mut NoResolver);
        assert!(r.is_err(), "a prefix of {cut} bytes loaded");
        assert_eq!(w.entities().count(), 0);
    }
    for at in 0..bytes.len() {
        let mut broken = bytes.clone();
        broken[at] ^= 0xff;
        let mut w = World::new();
        let r = reg.load_scene_binary(&broken, &mut w, &mut NoResolver);
        if r.is_err() {
            assert_eq!(w.entities().count(), 0);
        }
    }
}

#[test]
fn bytes_that_are_neither_form_say_so() {
    let mut world = World::new();
    let err = registry()
        .load_scene_bytes(&[b'{', 0xff, 0xfe], &mut world, &mut NoResolver)
        .unwrap_err();
    assert_eq!(err.issues()[0].offset, Some(1));
}

/// A generated scene of `n` entities: a tree ten wide, each with a
/// transform, a body and a name.
fn generated(n: usize) -> (World, SceneMap) {
    let mut world = World::new();
    let mut scene = SceneMap::new(None);
    let mut spawned = Vec::with_capacity(n);
    for i in 0..n {
        let entity = world.spawn();
        let at = Vec3::new(i as f32 * 0.37, (i % 17) as f32 * 1.5, -(i as f32) * 0.125);
        world.insert(
            entity,
            Transform {
                motor: Motor3::translation(at),
            },
        );
        world.insert(
            entity,
            RigidBody {
                motion: if i % 3 == 0 {
                    Motion::Static
                } else {
                    Motion::Dynamic
                },
                mass: 1.0 + (i % 50) as f32,
            },
        );
        if i > 0 {
            world.insert(entity, Parent(spawned[(i - 1) / 10]));
        }
        let mut e = SceneEntity::new(key(&format!("entity_{i}")), entity);
        e.name = Some(DisplayName::new(format!("Crate {}", i % 100)));
        scene.push(e).unwrap();
        spawned.push(entity);
    }
    (world, scene)
}

#[test]
fn a_big_scene_is_much_smaller_in_binary_and_reads_back_equal() {
    let reg = registry();
    let (world, scene) = generated(10_000);
    let text = reg.save_scene(&world, &scene);
    let bytes = reg.save_scene_binary(&world, &scene);
    println!(
        "10k entities: text {} bytes, binary {} bytes ({:.1}%)",
        text.len(),
        bytes.len(),
        100.0 * bytes.len() as f64 / text.len() as f64
    );
    assert!(bytes.len() * 3 < text.len());
    let mut again = World::new();
    let back = reg
        .load_scene_binary(&bytes, &mut again, &mut NoResolver)
        .unwrap();
    assert_eq!(reg.save_scene(&again, &back), text);
}
