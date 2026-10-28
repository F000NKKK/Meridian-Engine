//! `.mprefab`: instancing under ids `<instance>/<prefab id>`, whole-component
//! overrides, nesting, cycles, and every way a prefab can be wrong.

mod common;

use common::{Follow, Motion, RigidBody, key, registry, transform};
use meridian_data_substrate::{
    LoadError, MAX_PREFAB_DEPTH, NoResolver, Parent, PrefabInstance, PrefabLibrary, PrefabPath,
    SceneMap, SceneProblem,
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
fn an_instance_is_the_scene_entity_with_the_prefab_under_it() {
    let scene = r#"{ format: 1, entities: [
        { id: "world" },
        { id: "hero", name: "Player", prefab: "@prefab:actors/hero", parent: "@entity:world",
          components: { transform: { motor: { rotor: [1, 0, 0, 0], translation: [9, 0, 0] } } } },
    ] }"#;
    let (world, map) = load(scene, &mut library()).unwrap();
    assert_eq!(keys(&map), ["world", "hero", "hero/gun"]);

    let hero = map.entity(&key("hero")).unwrap();
    let gun = map.entity(&key("hero/gun")).unwrap();
    // The prefab root's components are the base; a listed component replaces the whole.
    assert_eq!(
        world.get::<RigidBody>(hero),
        Some(&RigidBody {
            motion: Motion::Dynamic,
            mass: 80.0
        })
    );
    assert_eq!(
        transform(&world, &map, "hero"),
        Motor3::translation(Vec3::new(9.0, 0.0, 0.0))
    );
    assert_eq!(
        map.get(&key("hero"))
            .unwrap()
            .name
            .as_ref()
            .map(|n| n.as_str()),
        Some("Player")
    );
    // The hierarchy and the references inside the prefab follow the instance.
    assert_eq!(world.get::<Parent>(gun), Some(&Parent(hero)));
    assert_eq!(
        world.get::<Parent>(hero),
        Some(&Parent(map.entity(&key("world")).unwrap()))
    );
    assert_eq!(world.get::<Follow>(gun).unwrap().target, key("hero"));
    assert_eq!(
        map.get(&key("hero/gun")).unwrap().generated_by,
        Some(key("hero"))
    );
    assert_eq!(
        map.get(&key("hero")).unwrap().prefab,
        Some(path("actors/hero"))
    );
}

#[test]
fn the_prefab_name_survives_unless_the_entity_has_its_own() {
    let scene = r#"{ format: 1, entities: [ { id: "h", prefab: "@prefab:actors/hero" } ] }"#;
    let (_, map) = load(scene, &mut library()).unwrap();
    assert_eq!(
        map.get(&key("h"))
            .unwrap()
            .name
            .as_ref()
            .map(|n| n.as_str()),
        Some("Body")
    );
}

#[test]
fn a_listed_component_replaces_the_prefabs_whole() {
    let scene = r#"{ format: 1, entities: [
        { id: "h", prefab: "@prefab:actors/hero",
          components: { rigid_body: { motion: "static", mass: 1 } } } ] }"#;
    let (world, map) = load(scene, &mut library()).unwrap();
    let body = world
        .get::<RigidBody>(map.entity(&key("h")).unwrap())
        .unwrap();
    assert_eq!(
        *body,
        RigidBody {
            motion: Motion::Static,
            mass: 1.0
        }
    );
    // The component it did not list is the prefab's.
    assert_eq!(
        transform(&world, &map, "h"),
        Motor3::translation(Vec3::new(0.0, 1.0, 0.0))
    );
}

#[test]
fn two_instances_do_not_share_ids_or_entities() {
    let scene = r#"{ format: 1, entities: [
        { id: "a", prefab: "@prefab:actors/hero" },
        { id: "b", prefab: "@prefab:actors/hero" } ] }"#;
    let (world, map) = load(scene, &mut library()).unwrap();
    assert_eq!(keys(&map), ["a", "a/gun", "b", "b/gun"]);
    let (ga, gb) = (
        map.entity(&key("a/gun")).unwrap(),
        map.entity(&key("b/gun")).unwrap(),
    );
    assert_ne!(ga, gb);
    assert_eq!(world.get::<Follow>(ga).unwrap().target, key("a"));
    assert_eq!(world.get::<Follow>(gb).unwrap().target, key("b"));
}

#[test]
fn prefabs_nest_and_ids_compose() {
    let mut lib = library();
    lib.insert(
        path("squads/pair"),
        r#"{ format: 1, entities: [
            { id: "root" },
            { id: "leader", parent: "@entity:root", prefab: "@prefab:actors/hero" } ] }"#,
    );
    let scene = r#"{ format: 1, entities: [ { id: "squad", prefab: "@prefab:squads/pair" } ] }"#;
    let (world, map) = load(scene, &mut lib).unwrap();
    assert_eq!(keys(&map), ["squad", "squad/leader", "squad/leader/gun"]);
    let gun = map.entity(&key("squad/leader/gun")).unwrap();
    assert_eq!(
        world.get::<Follow>(gun).unwrap().target,
        key("squad/leader")
    );
    let leader = map.entity(&key("squad/leader")).unwrap();
    assert_eq!(
        world.get::<Parent>(leader),
        Some(&Parent(map.entity(&key("squad")).unwrap()))
    );
}

#[test]
fn a_prefab_may_hold_a_reference_to_its_own_root() {
    let mut lib = PrefabLibrary::new();
    lib.insert(
        path("p"),
        r#"{ format: 1, entities: [
            { id: "r" }, { id: "c", parent: "@entity:r", components: { follow: { target: "@entity:r" } } } ] }"#,
    );
    let (world, map) = load(
        r#"{ format: 1, entities: [ { id: "x", prefab: "@prefab:p" } ] }"#,
        &mut lib,
    )
    .unwrap();
    assert_eq!(
        world
            .get::<Follow>(map.entity(&key("x/c")).unwrap())
            .unwrap()
            .target,
        key("x")
    );
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
fn the_same_prefab_twice_in_a_row_is_not_a_cycle() {
    let mut lib = library();
    lib.insert(
        path("twins"),
        r#"{ format: 1, entities: [
            { id: "r" },
            { id: "x", parent: "@entity:r", prefab: "@prefab:actors/hero" },
            { id: "y", parent: "@entity:r", prefab: "@prefab:actors/hero" } ] }"#,
    );
    let (_, map) = load(
        r#"{ format: 1, entities: [ { id: "t", prefab: "@prefab:twins" } ] }"#,
        &mut lib,
    )
    .unwrap();
    assert_eq!(keys(&map), ["t", "t/x", "t/x/gun", "t/y", "t/y/gun"]);
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
        matches!(&issue.problem, SceneProblem::DuplicateEntity { key, first: 0 } if key.as_str() == "hero/gun")
    );
    assert_eq!(issue.path.to_string(), "entities[1].prefab");
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
fn instantiating_from_code_names_the_instance() {
    let mut world = World::new();
    let map = registry()
        .instantiate_prefab(
            &PrefabInstance::new(key("npc_1"), path("actors/hero")),
            &mut world,
            &mut NoResolver,
            &mut library(),
        )
        .unwrap();
    assert_eq!(keys(&map), ["npc_1", "npc_1/gun"]);
    assert_eq!(world.entities().count(), 2);
    let mut world = World::new();
    let err = registry()
        .instantiate_prefab(
            &PrefabInstance::new(key("n"), path("nope")),
            &mut world,
            &mut NoResolver,
            &mut library(),
        )
        .unwrap_err();
    assert_eq!(err.issues()[0].path.to_string(), "entities[0].prefab");
    assert_eq!(world.entities().count(), 0);
}

#[test]
fn a_prefab_loads_as_itself_and_saves_in_both_forms() {
    let reg = registry();
    let mut world = World::new();
    let map = reg
        .load_prefab(HERO.as_bytes(), &mut world, &mut NoResolver, &mut library())
        .unwrap();
    assert_eq!(keys(&map), ["body", "gun"]);
    assert!(map.name().is_none());
    let text = reg.save_prefab(&world, &map);
    assert!(
        substrate_data::text::parse(&text)
            .unwrap()
            .value
            .get("name")
            .is_none(),
        "{text}"
    );
    let bytes = reg.save_prefab_binary(&world, &map);
    for source in [text.as_bytes(), bytes.as_slice()] {
        let mut again = World::new();
        let back = reg
            .load_prefab(source, &mut again, &mut NoResolver, &mut library())
            .unwrap();
        assert_eq!(reg.save_prefab(&again, &back), text);
    }
}

#[test]
fn saving_an_instanced_scene_leaves_generated_entities_to_the_prefab() {
    let reg = registry();
    let scene = r#"{ format: 1, entities: [
        { id: "hero", prefab: "@prefab:actors/hero",
          components: { rigid_body: { motion: "static", mass: 2 } } } ] }"#;
    let (world, map) = load(scene, &mut library()).unwrap();
    let saved = reg.save_scene(&world, &map);
    assert!(!saved.contains("hero/gun"), "{saved}");
    assert!(saved.contains("@prefab:actors/hero"), "{saved}");
    // Loading what was saved gives the same world shape.
    let (world2, map2) = load(&saved, &mut library()).unwrap();
    assert_eq!(keys(&map2), ["hero", "hero/gun"]);
    assert_eq!(reg.save_scene(&world2, &map2), saved);
    // The binary form too.
    let mut world3 = World::new();
    let map3 = reg
        .load_scene_with_prefabs(
            &reg.save_scene_binary(&world, &map),
            &mut world3,
            &mut NoResolver,
            &mut library(),
        )
        .unwrap();
    assert_eq!(keys(&map3), ["hero", "hero/gun"]);
}

#[test]
fn a_prefab_in_binary_is_instanced_like_text() {
    let reg = registry();
    let mut world = World::new();
    let map = reg
        .load_prefab(
            HERO.as_bytes(),
            &mut world,
            &mut NoResolver,
            &mut PrefabLibrary::new(),
        )
        .unwrap();
    let mut lib = PrefabLibrary::new();
    lib.insert(path("actors/hero"), reg.save_prefab_binary(&world, &map));
    let (_, map) = load(
        r#"{ format: 1, entities: [ { id: "h", prefab: "@prefab:actors/hero" } ] }"#,
        &mut lib,
    )
    .unwrap();
    assert_eq!(keys(&map), ["h", "h/gun"]);
}
