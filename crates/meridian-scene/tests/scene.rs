use meridian_ecs_core::{Component, Transform, World};
use meridian_gac_core::{Motor3, Rotor, Vec3};
use meridian_scene::{
    ComponentRegistry, EntityKind, NoResolver, Parent, SceneMap, TransformCodec, TransformData,
};
use substrate_data::with::RefTo;
use substrate_data::{Path, Ref, Resolve, Schema};

#[derive(Debug, Clone, Copy, PartialEq, Schema)]
enum Motion {
    #[data(rename = "dynamic")]
    Dynamic,
    #[data(rename = "static")]
    Static,
}

/// A body the physics moves.
#[derive(Debug, Clone, PartialEq, Schema)]
struct RigidBody {
    motion: Motion,
    /// Kilograms.
    mass: f32,
}
impl Component for RigidBody {}

#[derive(Debug, Clone, PartialEq, Schema)]
struct Follow {
    #[data(with = "RefTo<EntityKind>")]
    target: String,
}
impl Component for Follow {}

fn registry() -> ComponentRegistry {
    let mut r = ComponentRegistry::new();
    r.register_with::<Transform, TransformCodec>("transform")
        .register::<RigidBody>("rigid_body")
        .register::<Follow>("follow");
    r
}

const LEVEL: &str = r#"// A hand-written level.
{
  format: 1,
  name: "Level 1",
  entities: [
    {
      id: "world", // the root
      components: {
        transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 0, 0] } },
      },
    },
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

fn transform(world: &World, scene: &SceneMap, id: &str) -> Motor3 {
    world
        .get::<Transform>(scene.entity(id).unwrap())
        .unwrap()
        .motor
}

#[test]
fn loads_a_commented_scene() {
    let mut world = World::new();
    let scene = registry()
        .load_scene(LEVEL, &mut world, &mut NoResolver)
        .unwrap();
    assert_eq!(scene.name.as_deref(), Some("Level 1"));
    let ids: Vec<&str> = scene.entities.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["world", "hero", "camera"]);
    assert_eq!(
        transform(&world, &scene, "hero"),
        Motor3::translation(Vec3::new(0.0, 1.5, 0.0))
    );
    let hero = scene.entity("hero").unwrap();
    assert_eq!(
        world.get::<RigidBody>(hero),
        Some(&RigidBody {
            motion: Motion::Dynamic,
            mass: 80.0
        })
    );
    assert_eq!(scene.get("hero").unwrap().name.as_deref(), Some("Hero"));
    assert_eq!(scene.parent_of(hero), scene.entity("world"));
    let camera = scene.entity("camera").unwrap();
    assert_eq!(scene.parent_of(camera), Some(hero));
    assert_eq!(world.get::<Parent>(camera), Some(&Parent(hero)));
    assert_eq!(
        world.get::<Parent>(hero),
        scene.entity("world").map(Parent).as_ref()
    );
    assert!(
        world
            .get::<Parent>(scene.entity("world").unwrap())
            .is_none()
    );
    assert_eq!(world.get::<Follow>(camera).unwrap().target, "hero");
    assert!(world.get::<Transform>(camera).is_none());
}

#[test]
fn every_problem_is_reported_at_its_line_and_column_and_nothing_spawns() {
    let text = r#"{
  format: 1,
  entities: [
    { id: "a", components: { transform: { motor: { rotor: [1, 0, 0] } } } },
    { id: "b", parent: "@entity:nobody" },
    // a typo:
    { id: "c", components: { trasform: {} } },
    { id: "a", components: { rigid_body: { motion: "flying", mass: 1 } } },
  ],
}"#;
    let mut world = World::new();
    let errors = registry()
        .load_scene(text, &mut world, &mut NoResolver)
        .unwrap_err();
    let lines: Vec<String> = errors.0.iter().map(ToString::to_string).collect();
    assert_eq!(
        lines,
        [
            "4:59: entities[0].components.transform.motor.rotor: expected 4 items, found 3",
            "5:24: entities[1].parent: no entity `nobody` in this scene",
            "7:30: entities[2].components.trasform: unknown component `trasform` — did you mean `transform`?",
            "8:11: entities[3].id: duplicate entity id `a` (first at entities[0])",
            "8:52: entities[3].components.rigid_body.motion: unknown `flying` (expected `dynamic`, `static`)",
        ],
        "{errors}"
    );
    assert_eq!(
        world.query::<Transform>().count(),
        0,
        "a failed load spawns nothing"
    );
}

#[test]
fn syntax_and_format_errors_are_located() {
    let mut world = World::new();
    let e = registry().load_scene(
        "{\n  format: 1,\n  entities: [ { id: } ]\n}",
        &mut world,
        &mut NoResolver,
    );
    let e = e.unwrap_err().to_string();
    assert!(e.starts_with("3:21"), "{e}");
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
    assert!(e.to_string().contains("missing `format`"), "{e}");
}

#[test]
fn unknown_component_names_get_a_suggestion_or_pass_through() {
    let text =
        r#"{ format: 1, entities: [ { id: "x", components: { rigidbody: { mass: 2 } } } ] }"#;
    let mut world = World::new();
    let e = registry()
        .load_scene(text, &mut world, &mut NoResolver)
        .unwrap_err();
    assert_eq!(
        e.to_string(),
        "1:51: entities[0].components.rigidbody: unknown component `rigidbody` — did you mean `rigid_body`?"
    );

    let mut editor = registry();
    editor.allow_unknown(true);
    let scene = editor
        .load_scene(text, &mut world, &mut NoResolver)
        .unwrap();
    assert_eq!(scene.entities[0].unknown.len(), 1);
    let saved = editor.save_scene(&world, &scene);
    assert!(saved.contains("\"rigidbody\""), "{saved}");
    let again = editor
        .load_scene(&saved, &mut World::new(), &mut NoResolver)
        .unwrap();
    assert_eq!(again.entities[0].unknown, scene.entities[0].unknown);
}

#[test]
fn save_then_load_round_trips_bit_exactly() {
    let mut world = World::new();
    let mut scene = SceneMap::new();
    scene.name = Some("@generated".into());
    let root = world.spawn();
    let general = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 0.7),
        Vec3::new(0.1, 1e-7, -0.0),
    )
    .compose(Motor3::rotation(Vec3::Y, 0.3));
    world.insert(root, Transform { motor: general });
    scene.push("root", root);
    let child = world.spawn();
    world.insert(
        child,
        Transform {
            motor: Motor3::translation(Vec3::new(-0.0, 0.1, 1e-7)),
        },
    );
    world.insert(
        child,
        RigidBody {
            motion: Motion::Static,
            mass: 0.1,
        },
    );
    world.insert(
        child,
        Follow {
            target: "root".into(),
        },
    );
    scene.push("child", child).parent = Some("root".into());

    let registry = registry();
    let text = registry.save_scene(&world, &scene);
    let mut loaded_world = World::new();
    let loaded = registry
        .load_scene(&text, &mut loaded_world, &mut NoResolver)
        .unwrap();
    assert_eq!(loaded.name.as_deref(), Some("@generated"));
    for id in ["root", "child"] {
        let (a, b) = (
            transform(&world, &scene, id),
            transform(&loaded_world, &loaded, id),
        );
        let bits = |m: Motor3| m.0.0.map(f32::to_bits);
        assert_eq!(bits(a), bits(b), "{id}\n{text}");
    }
    let child2 = loaded.entity("child").unwrap();
    assert_eq!(loaded.parent_of(child2), loaded.entity("root"));
    assert_eq!(
        loaded_world.get::<RigidBody>(child2),
        world.get::<RigidBody>(child)
    );
    assert_eq!(
        loaded_world.get::<Follow>(child2),
        world.get::<Follow>(child)
    );
    // Saving what was loaded writes the same text.
    assert_eq!(registry.save_scene(&loaded_world, &loaded), text);
}

#[test]
fn outside_references_go_through_the_callers_resolver() {
    struct Prefabs(Vec<String>);
    impl Resolve for Prefabs {
        fn reference(&mut self, r: &Ref, _: &Path) -> Result<String, String> {
            self.0.push(format!("{}:{}", r.kind, r.id));
            if r.id == "actors/hero" {
                Ok(r.id.clone())
            } else {
                Err(format!("no prefab `{}`", r.id))
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
    assert_eq!(
        e.to_string(),
        "3:22: entities[1].prefab: no prefab `actors/villain`"
    );
    assert_eq!(
        seen.0,
        ["prefab:actors/hero", "prefab:actors/villain"],
        "entity refs stay in the document"
    );
}

#[test]
fn describe_lists_every_component_shape_by_name() {
    let d = registry().describe();
    let names: Vec<&str> = d.as_map().unwrap().keys().collect();
    assert_eq!(names, ["transform", "rigid_body", "follow"]);
    let transform = d.get("transform").unwrap();
    let motor = transform
        .get("defs")
        .and_then(|d| d.get("transform"))
        .and_then(|t| t.get("fields"))
        .and_then(|f| f.as_list())
        .and_then(|f| f.first())
        .unwrap();
    assert_eq!(motor.get("name").and_then(|n| n.as_str()), Some("motor"));
    let shape = motor.get("shape").unwrap();
    assert_eq!(shape.get("kind").and_then(|k| k.as_str()), Some("named"));
    assert_eq!(shape.get("name").and_then(|k| k.as_str()), Some("motor"));
    let body = d
        .get("rigid_body")
        .unwrap()
        .get("defs")
        .and_then(|d| d.get("RigidBody"))
        .unwrap();
    assert_eq!(
        body.get("doc").and_then(|n| n.as_str()),
        Some("A body the physics moves.")
    );
}

#[test]
fn the_registry_knows_types_by_name() {
    let r = registry();
    assert_eq!(r.name_of::<Transform>(), Some("transform"));
    assert_eq!(r.name_of::<Follow>(), Some("follow"));
    assert_eq!(r.name_of::<Parent>(), None);
    let t = TransformData(Motor3::translation(Vec3::new(1.0, 2.0, 3.0)));
    let back: TransformData = substrate_data::from_text(&substrate_data::to_text(&t))
        .unwrap()
        .value;
    assert_eq!(Transform::from(back).motor, t.0);
}

#[test]
fn a_parent_component_set_in_code_is_saved_by_id() {
    let mut world = World::new();
    let (a, b) = (world.spawn(), world.spawn());
    world.insert(b, Parent(a));
    let mut scene = SceneMap::new();
    scene.push("a", a);
    scene.push("b", b);
    let text = registry().save_scene(&world, &scene);
    assert!(text.contains("\"@entity:a\""), "{text}");
    let loaded = registry()
        .load_scene(&text, &mut World::new(), &mut NoResolver)
        .unwrap();
    assert_eq!(loaded.get("b").unwrap().parent.as_deref(), Some("a"));
}

#[test]
#[should_panic(expected = "already registered")]
fn a_name_is_registered_once() {
    let mut r = registry();
    r.register::<RigidBody>("body");
}
