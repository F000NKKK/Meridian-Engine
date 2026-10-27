//! Loading `.mscene` documents: values, hierarchy, and every problem typed and placed.

mod common;

use common::{Follow, Motion, RigidBody, key, name, registry, transform};
use meridian_ecs_core::{Transform, World};
use meridian_gac_core::{Motor3, Vec3};
use meridian_scene::{NoResolver, Parent, SceneProblem, UnknownComponents};
use substrate_data::{Path, Problem, Ref, RefKey, Resolve, Unresolved};

const LEVEL: &str = r#"// A hand-written level.
{
  format: 1,
  name: "Level 1",
  entities: [
    { id: "world", components: { transform: { motor: { rotor: [1, 0, 0, 0] } } } }, // the root
    /* the player */
    {
      id: "hero",
      name: "Hero",
      parent: "@entity:world",
      components: {
        transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 1.5, 0] } },
        rigid_body: { motion: "dynamic", mass: 80 },
        $rigid_body: "a note for whoever edits this",
      },
    },
    { id: "camera", parent: "@entity:hero", components: { follow: { target: "@entity:hero" } } },
  ],
}
"#;

#[test]
fn loads_a_commented_scene() {
    let mut world = World::new();
    let scene = registry()
        .load_scene(LEVEL, &mut world, &mut NoResolver)
        .unwrap();
    assert_eq!(scene.name().map(|n| n.as_str()), Some("Level 1"));
    let keys: Vec<&str> = scene.entities().iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, ["world", "hero", "camera"]);
    assert_eq!(
        transform(&world, &scene, "hero"),
        Motor3::translation(Vec3::new(0.0, 1.5, 0.0))
    );
    let hero = scene.entity(&key("hero")).unwrap();
    let body = RigidBody {
        motion: Motion::Dynamic,
        mass: 80.0,
    };
    assert_eq!(world.get::<RigidBody>(hero), Some(&body));
    let camera = scene.entity(&key("camera")).unwrap();
    assert_eq!(world.get::<Follow>(camera).unwrap().target, key("hero"));
    assert!(world.get::<Transform>(camera).is_none());
}

#[test]
fn parents_become_parent_components() {
    let mut world = World::new();
    let scene = registry()
        .load_scene(LEVEL, &mut world, &mut NoResolver)
        .unwrap();
    let [root, hero, camera] = ["world", "hero", "camera"].map(|k| scene.entity(&key(k)).unwrap());
    assert_eq!(world.get::<Parent>(camera), Some(&Parent(hero)));
    assert_eq!(world.get::<Parent>(hero), Some(&Parent(root)));
    assert_eq!(world.get::<Parent>(root), None);
    assert_eq!(scene.parent_of(camera), Some(hero));
}

/// One problem of each kind, on known lines.
const BROKEN: &str = r#"{
  format: 1,
  entities: [
    { id: "a", components: { transform: { motor: { rotor: [1, 0, 0] } } } },
    { id: "b", parent: "@entity:nobody" },
    // a typo:
    { id: "c", components: { trasform: {} } },
    { id: "a", components: { rigid_body: { motion: "flying", mass: 1 } } },
  ],
}"#;

#[test]
fn every_problem_is_typed_placed_and_nothing_spawns() {
    let mut world = World::new();
    let e = registry()
        .load_scene(BROKEN, &mut world, &mut NoResolver)
        .unwrap_err();
    let at: Vec<_> = e
        .issues()
        .iter()
        .map(|i| (i.line.unwrap(), i.column.unwrap(), i.path.to_string()))
        .collect();
    let want = [
        (4, 59, "entities[0].components.transform.motor.rotor"),
        (5, 24, "entities[1].parent"),
        (7, 30, "entities[2].components.trasform"),
        (8, 11, "entities[3].id"),
        (8, 52, "entities[3].components.rigid_body.motion"),
    ];
    assert_eq!(at, want.map(|(l, c, p)| (l, c, p.to_owned())), "{e}");
    let problems: Vec<&SceneProblem> = e.issues().iter().map(|i| &i.problem).collect();
    assert!(matches!(
        problems[1],
        SceneProblem::Data {
            problem: Problem::Unresolved,
            ..
        }
    ));
    let typo = SceneProblem::UnknownComponent {
        name: name("trasform"),
        suggestion: Some(name("transform")),
    };
    assert_eq!(problems[2], &typo);
    assert_eq!(
        problems[3],
        &SceneProblem::DuplicateEntity {
            key: key("a"),
            first: 0
        }
    );
    assert_eq!(
        world.query::<Transform>().count(),
        0,
        "a failed load spawns nothing"
    );
}

#[test]
fn syntax_and_format_problems_are_placed() {
    let mut world = World::new();
    let e = registry()
        .load_scene(
            "{\n  format: 1,\n  entities: [ { id: } ]\n}",
            &mut world,
            &mut NoResolver,
        )
        .unwrap_err();
    let first = &e.issues()[0];
    assert_eq!((first.line, first.column), (Some(3), Some(21)));
    assert!(matches!(
        first.problem,
        SceneProblem::Data {
            problem: Problem::Syntax,
            ..
        }
    ));
    let e = registry()
        .load_scene("{ format: 2 }", &mut world, &mut NoResolver)
        .unwrap_err();
    assert_eq!(
        e.to_string(),
        "1:11: format: unsupported scene format 2 (this reads 1)"
    );
    let e = registry()
        .load_scene("{ entities: [] }", &mut world, &mut NoResolver)
        .unwrap_err();
    assert!(matches!(
        e.issues()[0].problem,
        SceneProblem::Data {
            problem: Problem::MissingField,
            ..
        }
    ));
}

#[test]
fn unknown_components_are_refused_or_kept() {
    let text = r#"{ format: 1, entities: [ { id: "x", components: { rigidbody: { mass: 2 }, "Bad Name": 1 } } ] }"#;
    let mut world = World::new();
    let e = registry()
        .load_scene(text, &mut world, &mut NoResolver)
        .unwrap_err();
    assert_eq!(
        e.issues()[0].to_string(),
        "1:51: entities[0].components.rigidbody: unknown component `rigidbody` — did you mean `rigid_body`?"
    );
    assert!(matches!(
        e.issues()[1].problem,
        SceneProblem::BadComponentName(_)
    ));

    let kept_text = text.replace(r#", "Bad Name": 1"#, "");
    let mut editor = registry();
    editor.set_unknown_components(UnknownComponents::Keep);
    let scene = editor
        .load_scene(&kept_text, &mut world, &mut NoResolver)
        .unwrap();
    assert_eq!(scene.entities()[0].kept[0].name, name("rigidbody"));
    let saved = editor.save_scene(&world, &scene);
    let again = editor
        .load_scene(&saved, &mut World::new(), &mut NoResolver)
        .unwrap();
    assert_eq!(again.entities()[0].kept, scene.entities()[0].kept);
}

#[test]
fn outside_references_go_through_the_callers_resolver() {
    struct Prefabs(Vec<Ref>);
    impl Resolve for Prefabs {
        fn reference(&mut self, r: &Ref, _: &Path) -> Result<RefKey, Unresolved> {
            self.0.push(r.clone());
            if r.id == "actors/hero" {
                Ok(r.id.clone())
            } else {
                Err(Unresolved::Missing(r.clone()))
            }
        }
    }
    let text = r#"{ format: 1, entities: [
  { id: "a", prefab: "@prefab:actors/hero", parent: "@entity:b" },
  { id: "b", prefab: "@prefab:actors/villain" },
] }"#;
    let mut seen = Prefabs(Vec::new());
    let e = registry()
        .load_scene(text, &mut World::new(), &mut seen)
        .unwrap_err();
    let issue = &e.issues()[0];
    assert_eq!(
        (issue.line, issue.path.to_string()),
        (Some(3), "entities[1].prefab".to_owned())
    );
    assert!(matches!(
        issue.problem,
        SceneProblem::Data {
            problem: Problem::Unresolved,
            ..
        }
    ));
    let kinds: Vec<&str> = seen.0.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(
        kinds,
        ["prefab", "prefab"],
        "entity references stay in the document"
    );
}
