//! `.mprefab` problems: cycles, depth, a missing prefab, a prefab that is
//! wrong, and ids that collide.

mod common;

use common::{key, registry, transform};
use meridian_data_substrate::{
    LoadError, MAX_PREFAB_DEPTH, NoResolver, PrefabLibrary, PrefabPath, SceneMap, SceneProblem,
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

/// Every problem of an error, with the errors of invalid prefabs flattened in.
fn problems(error: &LoadError) -> Vec<&SceneProblem> {
    let mut all = Vec::new();
    for issue in error.issues() {
        all.push(&issue.problem);
        if let SceneProblem::PrefabInvalid { error, .. } = &issue.problem {
            all.extend(problems(error));
        }
    }
    all
}

#[test]
fn a_prefab_that_includes_itself_is_a_cycle() {
    let mut lib = PrefabLibrary::new();
    lib.insert(path("a"), r#"{ format: 1, entities: [ { id: "r" }, { id: "c", parent: "@entity:r", prefab: "@prefab:a" } ] }"#);
    let err = load(
        r#"{ format: 1, entities: [ { id: "x", prefab: "@prefab:a" } ] }"#,
        &mut lib,
    )
    .unwrap_err();
    let cycle = problems(&err).into_iter().find_map(|p| match p {
        SceneProblem::PrefabCycle { chain } => Some(chain.clone()),
        _ => None,
    });
    assert_eq!(cycle, Some(vec![path("a"), path("a")]));
}

#[test]
fn prefabs_that_include_each_other_are_a_cycle_with_the_chain() {
    let mut lib = PrefabLibrary::new();
    lib.insert(
        path("a"),
        r#"{ format: 1, entities: [ { id: "r", prefab: "@prefab:b" } ] }"#,
    );
    lib.insert(
        path("b"),
        r#"{ format: 1, entities: [ { id: "r", prefab: "@prefab:a" } ] }"#,
    );
    let err = load(
        r#"{ format: 1, entities: [ { id: "x", prefab: "@prefab:a" } ] }"#,
        &mut lib,
    )
    .unwrap_err();
    let chain = problems(&err).into_iter().find_map(|p| match p {
        SceneProblem::PrefabCycle { chain } => Some(chain.clone()),
        _ => None,
    });
    assert_eq!(chain, Some(vec![path("a"), path("b"), path("a")]));
    let shown = err.to_string();
    assert!(shown.contains("a -> b -> a"), "{shown}");
}

#[test]
fn nesting_too_deep_is_refused() {
    let mut lib = PrefabLibrary::new();
    let depth = MAX_PREFAB_DEPTH + 4;
    for i in 0..depth {
        let body = if i + 1 < depth {
            format!(
                r#"{{ format: 1, entities: [ {{ id: "r", prefab: "@prefab:p{}" }} ] }}"#,
                i + 1
            )
        } else {
            r#"{ format: 1, entities: [ { id: "r" } ] }"#.to_owned()
        };
        lib.insert(path(&format!("p{i}")), body);
    }
    let err = load(
        r#"{ format: 1, entities: [ { id: "x", prefab: "@prefab:p0" } ] }"#,
        &mut lib,
    )
    .unwrap_err();
    assert!(
        problems(&err).iter().any(
            |p| matches!(p, SceneProblem::PrefabTooDeep { limit } if *limit == MAX_PREFAB_DEPTH)
        )
    );
}

#[test]
fn a_missing_prefab_says_where_and_suggests() {
    let scene = "{\n  format: 1,\n  entities: [\n    { id: \"a\" },\n    { id: \"h\", prefab: \"@prefab:actors/hreo\" },\n  ],\n}";
    let mut world = World::new();
    let err = registry()
        .load_scene_with_prefabs(
            scene.as_bytes(),
            &mut world,
            &mut NoResolver,
            &mut library(),
        )
        .unwrap_err();
    assert_eq!(world.entities().count(), 0, "all or nothing");
    let issue = &err.issues()[0];
    assert_eq!(issue.path.to_string(), "entities[1].prefab");
    assert_eq!((issue.line, issue.column), (Some(5), Some(24)));
    assert!(matches!(
        &issue.problem,
        SceneProblem::PrefabUnavailable { prefab, suggestion: Some(s), .. }
            if prefab == &path("actors/hreo") && s == &path("actors/hero")
    ));
    assert!(
        issue.to_string().contains("did you mean `actors/hero`?"),
        "{issue}"
    );
    // With nothing close there is no suggestion.
    let scene = r#"{ format: 1, entities: [ { id: "h", prefab: "@prefab:zzzzzzzz" } ] }"#;
    let err = load(scene, &mut library()).unwrap_err();
    assert!(matches!(
        &err.issues()[0].problem,
        SceneProblem::PrefabUnavailable {
            suggestion: None,
            ..
        }
    ));
}

#[test]
fn plain_load_scene_keeps_the_prefab_path_without_instancing() {
    let scene = r#"{ format: 1, entities: [ { id: "h", prefab: "@prefab:actors/hero" } ] }"#;
    let mut world = World::new();
    let map = registry()
        .load_scene(scene, &mut world, &mut NoResolver)
        .unwrap();
    assert_eq!(keys(&map), ["h"]);
    assert_eq!(
        map.get(&key("h")).unwrap().prefab,
        Some(path("actors/hero"))
    );
}

#[test]
fn a_failed_load_leaves_the_world_as_it_was() {
    let mut lib = library();
    let mut world = World::new();
    let reg = registry();
    let bad =
        br#"{ format: 1, entities: [ { id: "ok" }, { id: "x", prefab: "@prefab:actors/nope" } ] }"#;
    assert!(
        reg.load_scene_with_prefabs(bad, &mut world, &mut NoResolver, &mut lib)
            .is_err()
    );
    assert_eq!(world.entities().count(), 0);
    let good = br#"{ format: 1, entities: [ { id: "h", prefab: "@prefab:actors/hero" } ] }"#;
    let map = reg
        .load_scene_with_prefabs(good, &mut world, &mut NoResolver, &mut lib)
        .unwrap();
    assert_eq!(world.entities().count(), 2);
    assert_eq!(
        transform(&world, &map, "h"),
        Motor3::translation(Vec3::new(0.0, 1.0, 0.0))
    );
}
