//! Names below an instance that are wrong (a `parent`, a reference, an
//! override inside a prefab), and overrides in damaged binary documents.

mod common;

use common::{key, registry, transform};
use meridian_data_substrate::{
    LoadError, NoResolver, PrefabLibrary, PrefabPath, SceneIssue, SceneProblem,
};
use meridian_ecs::World;
use meridian_gac::{Motor3, Vec3};
use substrate_data::Problem;

const HERO: &str = r#"{
  format: 1,
  entities: [
    { id: "body", components: {
        transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 1, 0] } },
        rigid_body: { motion: "dynamic", mass: 80 } } },
    { id: "gun", parent: "@entity:body", components: {
        follow: { target: "@entity:body" } } },
  ],
}"#;

fn library() -> PrefabLibrary {
    let mut lib = PrefabLibrary::new();
    lib.insert(PrefabPath::new("actors/hero").unwrap(), HERO);
    lib
}

/// The error of loading `scene` with the hero prefab; nothing is spawned.
fn fail(scene: &str) -> LoadError {
    let mut world = World::new();
    let result = registry().load_scene_with_prefabs(
        scene.as_bytes(),
        &mut world,
        &mut NoResolver,
        &mut library(),
    );
    assert_eq!(world.entities().count(), 0, "a failed load spawned");
    result.expect_err("the scene should not load")
}

fn only(error: &LoadError) -> &SceneIssue {
    assert_eq!(error.issues().len(), 1, "{error}");
    &error.issues()[0]
}

#[test]
fn names_below_an_instance_load_when_the_prefab_makes_them() {
    let scene = r#"{ format: 1, entities: [
        { id: "h", prefab: "@prefab:actors/hero" },
        { id: "pet", parent: "@entity:h/gun", components: {
            transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 0, 4] } },
            follow: { target: "@entity:h/gun" } } } ] }"#;
    let mut world = World::new();
    let map = registry()
        .load_scene_with_prefabs(
            scene.as_bytes(),
            &mut world,
            &mut NoResolver,
            &mut library(),
        )
        .unwrap();
    assert_eq!(
        transform(&world, &map, "pet"),
        Motor3::translation(Vec3::new(0.0, 0.0, 4.0))
    );
}

#[test]
fn a_parent_below_an_instance_must_exist() {
    let e = fail(
        r#"{ format: 1, entities: [
            { id: "h", prefab: "@prefab:actors/hero" },
            { id: "pet", parent: "@entity:h/gunn" } ] }"#,
    );
    let SceneProblem::NoSuchGeneratedEntity { suggestion, .. } = &only(&e).problem else {
        panic!("{e}");
    };
    assert_eq!(suggestion.as_ref(), Some(&key("h/gun")));
    assert_eq!(only(&e).path.to_string(), "entities[1].parent");
}

#[test]
fn a_reference_below_an_instance_must_exist() {
    let e = fail(
        r#"{ format: 1, entities: [
            { id: "h", prefab: "@prefab:actors/hero" },
            { id: "pet", components: { follow: { target: "@entity:h/zzz" } } } ] }"#,
    );
    assert!(
        matches!(
            &only(&e).problem,
            SceneProblem::NoSuchGeneratedEntity { .. }
        ),
        "{e}"
    );
    assert_eq!(
        only(&e).path.to_string(),
        "entities[1].components.follow.target"
    );
}

#[test]
fn a_name_below_an_entity_that_is_no_instance_is_still_unknown() {
    let e = fail(
        r#"{ format: 1, entities: [
            { id: "h" },
            { id: "pet", parent: "@entity:h/gun" } ] }"#,
    );
    assert!(
        matches!(
            &only(&e).problem,
            SceneProblem::Data {
                problem: Problem::Unresolved,
                ..
            }
        ),
        "{e}"
    );
}

#[test]
fn a_generated_parent_needs_the_prefabs_that_make_it() {
    let scene = r#"{ format: 1, entities: [
        { id: "h", prefab: "@prefab:actors/hero" },
        { id: "pet", parent: "@entity:h/gun" } ] }"#;
    let mut world = World::new();
    let e = registry()
        .load_scene(scene, &mut world, &mut NoResolver)
        .unwrap_err();
    assert!(
        matches!(
            &only(&e).problem,
            SceneProblem::NoSuchGeneratedEntity { .. }
        ),
        "{e}"
    );
}

#[test]
fn an_instance_cannot_be_parented_to_its_own_entity() {
    let e = fail(
        r#"{ format: 1, entities: [
            { id: "h", prefab: "@prefab:actors/hero", parent: "@entity:h/gun" } ] }"#,
    );
    assert!(
        matches!(&only(&e).problem, SceneProblem::ParentInsideInstance { .. }),
        "{e}"
    );
    assert_eq!(only(&e).path.to_string(), "entities[0].parent");
}

#[test]
fn a_bad_override_inside_a_prefab_is_that_prefabs_problem() {
    let mut lib = library();
    lib.insert(
        PrefabPath::new("squad").unwrap(),
        r#"{ format: 1, entities: [
            { id: "root" },
            { id: "leader", parent: "@entity:root", prefab: "@prefab:actors/hero",
              overrides: { "leader/zzz": { remove: ["follow"] } } } ] }"#,
    );
    let mut world = World::new();
    let e = registry()
        .load_scene_with_prefabs(
            br#"{ format: 1, entities: [ { id: "s", prefab: "@prefab:squad" } ] }"#,
            &mut world,
            &mut NoResolver,
            &mut lib,
        )
        .unwrap_err();
    let SceneProblem::PrefabInvalid { error, .. } = &only(&e).problem else {
        panic!("{e}");
    };
    assert!(
        matches!(&only(error).problem, SceneProblem::NoSuchGeneratedEntity { key: k, .. } if *k == key("s/leader/zzz"))
    );
    assert_eq!(only(&e).path.to_string(), "entities[0].prefab");
}

#[test]
fn malformed_binary_overrides_are_errors_never_panics() {
    let scene = r#"{ format: 1, entities: [
        { id: "h", prefab: "@prefab:actors/hero", overrides: {
            h: { fields: { rigid_body: { mass: 2 } }, remove: ["follow"] },
            "h/gun": { components: { follow: { target: "@entity:h" } } } } } ] }"#;
    let value = substrate_data::text::parse(scene).unwrap().value;
    let bytes = substrate_data::binary::encode(&value);
    for len in 0..bytes.len() {
        let mut world = World::new();
        let result = registry().load_scene_with_prefabs(
            &bytes[..len],
            &mut world,
            &mut NoResolver,
            &mut library(),
        );
        assert!(result.is_err(), "{len} bytes of {} loaded", bytes.len());
        assert_eq!(world.entities().count(), 0);
    }
    // A bit flipped anywhere is an error or a document, not a panic.
    for at in 0..bytes.len() {
        let mut flipped = bytes.clone();
        flipped[at] ^= 0xFF;
        let mut world = World::new();
        let result = registry().load_scene_with_prefabs(
            &flipped,
            &mut world,
            &mut NoResolver,
            &mut library(),
        );
        if result.is_err() {
            assert_eq!(world.entities().count(), 0);
        }
    }
}
