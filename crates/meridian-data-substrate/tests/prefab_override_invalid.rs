//! Overrides that are wrong: every problem typed, with its path, and
//! nothing spawned.

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

/// `fail` for one instance `h` with `overrides` and nothing else wrong.
fn fail_overrides(overrides: &str) -> LoadError {
    fail(&format!(
        r#"{{ format: 1, entities: [
            {{ id: "h", prefab: "@prefab:actors/hero", overrides: {overrides} }} ] }}"#
    ))
}

fn only(error: &LoadError) -> &SceneIssue {
    assert_eq!(error.issues().len(), 1, "{error}");
    &error.issues()[0]
}

#[test]
fn the_same_overrides_spelled_right_load() {
    let scene = r#"{ format: 1, entities: [ { id: "h", prefab: "@prefab:actors/hero",
        overrides: { h: { fields: { transform: { motor: { translation: [2, 0, 0] } } } },
                     "h/gun": { remove: ["follow"] } } } ] }"#;
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
        transform(&world, &map, "h"),
        Motor3::translation(Vec3::new(2.0, 0.0, 0.0))
    );
}

#[test]
fn an_unknown_field_is_placed_at_the_field() {
    let e = fail_overrides(r#"{ h: { fields: { rigid_body: { mas: 2 } } } }"#);
    let issue = only(&e);
    assert!(matches!(
        issue.problem,
        SceneProblem::Data {
            problem: Problem::UnknownField,
            ..
        }
    ));
    assert_eq!(
        issue.path.to_string(),
        "entities[0].overrides.h.fields.rigid_body.mas"
    );
    assert!(issue.line.is_some() && issue.column.is_some());
}

#[test]
fn a_field_of_the_wrong_type_is_placed_at_the_field() {
    let e = fail_overrides(r#"{ "h/gun": { fields: { follow: { target: 3 } } } }"#);
    let issue = only(&e);
    assert!(matches!(
        issue.problem,
        SceneProblem::Data {
            problem: Problem::Expected,
            ..
        }
    ));
    assert!(
        issue.path.to_string().ends_with("fields.follow.target"),
        "{e}"
    );
}

#[test]
fn an_unknown_component_suggests_the_registered_one() {
    let e = fail_overrides(r#"{ h: { fields: { rigidbody: { mass: 1 } } } }"#);
    let SceneProblem::UnknownComponent { name, suggestion } = &only(&e).problem else {
        panic!("{e}");
    };
    assert_eq!(name.as_str(), "rigidbody");
    assert_eq!(suggestion.as_ref().map(|s| s.as_str()), Some("rigid_body"));
    assert_eq!(
        only(&e).path.to_string(),
        "entities[0].overrides.h.fields.rigidbody"
    );
}

#[test]
fn a_target_the_prefab_does_not_make_suggests_the_closest() {
    let e = fail_overrides(r#"{ "h/gunn": { fields: { follow: { target: "@entity:h" } } } }"#);
    let SceneProblem::NoSuchGeneratedEntity {
        key: k,
        instance,
        suggestion,
    } = &only(&e).problem
    else {
        panic!("{e}");
    };
    assert_eq!((k, instance), (&key("h/gunn"), &key("h")));
    assert_eq!(suggestion.as_ref(), Some(&key("h/gun")));
    assert!(only(&e).path.to_string().contains("h/gunn"), "{e}");
}

#[test]
fn a_target_outside_the_instance_is_refused() {
    for target in ["other", "hh/gun", "gun"] {
        let e = fail_overrides(&format!(r#"{{ "{target}": {{ remove: ["follow"] }} }}"#));
        assert!(
            matches!(&only(&e).problem, SceneProblem::OverrideOutsideInstance { instance, .. } if *instance == key("h")),
            "{target}: {e}"
        );
    }
}

#[test]
fn a_target_that_is_not_an_entity_key_is_refused() {
    let e = fail_overrides(r#"{ "has space": { remove: ["follow"] } }"#);
    assert!(
        matches!(&only(&e).problem, SceneProblem::BadOverrideTarget(_)),
        "{e}"
    );
}

#[test]
fn removing_what_is_not_there_is_refused() {
    let e = fail_overrides(r#"{ "h/gun": { remove: ["rigid_body"] } }"#);
    let SceneProblem::OverrideRemovesMissing { target, name } = &only(&e).problem else {
        panic!("{e}");
    };
    assert_eq!((target, name.as_str()), (&key("h/gun"), "rigid_body"));
    assert_eq!(
        only(&e).path.to_string(),
        r#"entities[0].overrides["h/gun"].remove[0]"#
    );
}

#[test]
fn overrides_without_a_prefab_are_refused() {
    let e = fail(
        r#"{ format: 1, entities: [ { id: "x", overrides: { x: { remove: ["follow"] } } } ] }"#,
    );
    assert!(
        matches!(&only(&e).problem, SceneProblem::OverridesWithoutPrefab { key: k } if *k == key("x"))
    );
    assert_eq!(only(&e).path.to_string(), "entities[0].overrides");
}

#[test]
fn overrides_need_a_prefab_source_to_load() {
    let scene = r#"{ format: 1, entities: [
        { id: "h", prefab: "@prefab:actors/hero", overrides: { h: { remove: ["follow"] } } } ] }"#;
    let mut world = World::new();
    let e = registry()
        .load_scene(scene, &mut world, &mut NoResolver)
        .unwrap_err();
    assert!(
        matches!(&only(&e).problem, SceneProblem::OverridesNeedPrefabs { key: k } if *k == key("h"))
    );
    assert_eq!(world.entities().count(), 0);
}

#[test]
fn the_shape_of_overrides_is_checked() {
    for (overrides, path) in [
        (r#"[1]"#, "entities[0].overrides"),
        (r#"{ h: 3 }"#, "entities[0].overrides.h"),
        (r#"{ h: { field: {} } }"#, "entities[0].overrides.h.field"),
        (r#"{ h: { fields: [] } }"#, "entities[0].overrides.h.fields"),
        (
            r#"{ h: { components: 1 } }"#,
            "entities[0].overrides.h.components",
        ),
        (
            r#"{ h: { remove: "follow" } }"#,
            "entities[0].overrides.h.remove",
        ),
        (
            r#"{ h: { remove: [3] } }"#,
            "entities[0].overrides.h.remove[0]",
        ),
        (
            r#"{ h: { remove: ["Not A Name"] } }"#,
            "entities[0].overrides.h.remove[0]",
        ),
        (
            r#"{ h: { fields: { "Bad Name": {} } } }"#,
            r#"entities[0].overrides.h.fields["Bad Name"]"#,
        ),
    ] {
        let e = fail_overrides(overrides);
        assert_eq!(only(&e).path.to_string(), path, "{overrides}: {e}");
    }
}

#[test]
fn every_problem_is_reported_together_and_a_target_once() {
    let e = fail_overrides(
        r#"{ "h/nope": { fields: { follow: {}, rigid_body: {} }, remove: ["follow"] },
             h: { fields: { rigid_body: { mas: 1 } } } }"#,
    );
    let missing = (e.issues().iter())
        .filter(|i| matches!(i.problem, SceneProblem::NoSuchGeneratedEntity { .. }))
        .count();
    assert_eq!((e.issues().len(), missing), (2, 1), "{e}");
}
