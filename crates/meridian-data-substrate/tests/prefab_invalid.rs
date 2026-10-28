//! `.mprefab` that is wrong: problems inside it, its shape, and ids that collide.

mod common;

use common::{key, registry, transform};
use meridian_data_substrate::{
    LoadError, NoResolver, PrefabLibrary, PrefabPath, SceneMap, SceneProblem,
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
fn a_prefab_with_problems_reports_them_inside_the_prefab() {
    let mut lib = library();
    lib.insert(
        path("bad"),
        "{ format: 1, entities: [ { id: \"r\", components: { trasform: {} } } ] }",
    );
    let scene = r#"{ format: 1, entities: [ { id: "x", prefab: "@prefab:bad" } ] }"#;
    let err = load(scene, &mut lib).unwrap_err();
    let SceneProblem::PrefabInvalid { prefab, error } = &err.issues()[0].problem else {
        panic!("{err}");
    };
    assert_eq!(prefab, &path("bad"));
    assert_eq!(
        error.issues()[0].path.to_string(),
        "entities[0].components.trasform"
    );
    assert!(matches!(
        &error.issues()[0].problem,
        SceneProblem::UnknownComponent {
            suggestion: Some(_),
            ..
        }
    ));
    assert!(
        err.to_string().contains("prefab `bad` doesn't load"),
        "{err}"
    );
    // Bytes that are not a document at all are a problem of the prefab too.
    lib.insert(path("junk"), vec![0xff, 0xfe]);
    let err = load(
        r#"{ format: 1, entities: [ { id: "x", prefab: "@prefab:junk" } ] }"#,
        &mut lib,
    )
    .unwrap_err();
    assert!(matches!(
        &err.issues()[0].problem,
        SceneProblem::PrefabInvalid { .. }
    ));
}

#[test]
fn the_shape_of_a_prefab_is_checked() {
    let mut lib = PrefabLibrary::new();
    lib.insert(path("empty"), "{ format: 1, entities: [] }");
    lib.insert(path("rooted"), r#"{ format: 1, entities: [ { id: "a", parent: "@entity:b" }, { id: "b", parent: "@entity:a" } ] }"#);
    lib.insert(
        path("orphan"),
        r#"{ format: 1, entities: [ { id: "a" }, { id: "b" } ] }"#,
    );
    lib.insert(
        path("named"),
        r#"{ format: 1, name: "no", entities: [ { id: "a" } ] }"#,
    );
    for (name, expected) in [
        ("empty", "PrefabEmpty"),
        ("rooted", "PrefabRootHasParent"),
        ("orphan", "PrefabOrphan"),
    ] {
        let scene =
            format!(r#"{{ format: 1, entities: [ {{ id: "x", prefab: "@prefab:{name}" }} ] }}"#);
        let err = load(&scene, &mut lib).unwrap_err();
        let found = problems(&err)
            .into_iter()
            .map(|p| format!("{p:?}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(found.contains(expected), "{name}: {found}");
    }
    // `name` is a scene's key, not a prefab's.
    let err = load(
        r#"{ format: 1, entities: [ { id: "x", prefab: "@prefab:named" } ] }"#,
        &mut lib,
    )
    .unwrap_err();
    assert!(err.to_string().contains("name"), "{err}");
}

#[test]
fn an_id_an_instance_makes_cannot_be_used_again() {
    let scene = r#"{ format: 1, entities: [
        { id: "hero/gun" },
        { id: "hero", prefab: "@prefab:actors/hero" } ] }"#;
    let err = load(scene, &mut library()).unwrap_err();
    let issue = &err.issues()[0];
    assert!(
        matches!(&issue.problem, SceneProblem::DuplicateEntity { key: duplicate, first: 0 } if duplicate == &key("hero/gun"))
    );
    assert_eq!(issue.path.to_string(), "entities[1].prefab");
}

#[test]
fn a_prefab_of_the_right_shape_loads() {
    let mut lib = PrefabLibrary::new();
    lib.insert(path("tree"), r#"{ format: 1, entities: [ { id: "r", components: { transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 2, 0] } } } }, { id: "c", parent: "@entity:r" } ] }"#);
    let scene = r#"{ format: 1, entities: [ { id: "t", prefab: "@prefab:tree" } ] }"#;
    let (world, map) = load(scene, &mut lib).unwrap();
    assert_eq!(map.entities().len(), 2);
    assert_eq!(
        transform(&world, &map, "t"),
        Motor3::translation(Vec3::new(0.0, 2.0, 0.0))
    );
}
