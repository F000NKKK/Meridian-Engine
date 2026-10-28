//! Saving scenes, round trips, and what the registry says of itself.

mod common;

use common::{Follow, Motion, RigidBody, key, name, registry, transform};
use meridian_data_substrate::{
    ComponentName, DisplayName, NoResolver, Parent, RegisterError, SceneEntity, SceneMap,
    TransformCodec, TransformForm,
};
use meridian_ecs::{Transform, World};
use meridian_gac::{Motor3, Rotor, Vec3};
use substrate_data::schema::{DefinitionDoc, ShapeNode};
use substrate_data::{Schema, Shape};

fn general_motor() -> Motor3 {
    let r = Rotor::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 0.7);
    Motor3::from_rotation_translation(r, Vec3::new(0.1, 1e-7, -0.0))
        .compose(Motor3::rotation(Vec3::Y, 0.3))
}

/// A two-entity world built in code, and its scene map.
fn built_world() -> (World, SceneMap) {
    let mut world = World::new();
    let mut scene = SceneMap::new(Some(DisplayName::new("@generated")));
    let root = world.spawn();
    world.insert(root, Transform::from_motor(general_motor()));
    scene.push(SceneEntity::new(key("root"), root)).unwrap();
    let child = world.spawn();
    world.insert(
        child,
        Transform::from_motor(Motor3::translation(Vec3::new(-0.0, 0.1, 1e-7))),
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
            target: key("root"),
        },
    );
    world.insert(child, Parent(root));
    scene.push(SceneEntity::new(key("child"), child)).unwrap();
    (world, scene)
}

#[test]
fn save_then_load_round_trips_bit_exactly() {
    let (world, scene) = built_world();
    let registry = registry();
    let text = registry.save_scene(&world, &scene);
    assert!(text.contains("\"@entity:root\""), "{text}");
    let mut loaded_world = World::new();
    let loaded = registry
        .load_scene(&text, &mut loaded_world, &mut NoResolver)
        .unwrap();
    assert_eq!(loaded.name(), scene.name());
    for k in ["root", "child"] {
        let bits = |m: Motor3| m.0.0.map(f32::to_bits);
        let (a, b) = (
            transform(&world, &scene, k),
            transform(&loaded_world, &loaded, k),
        );
        assert_eq!(bits(a), bits(b), "{k}\n{text}");
    }
    let [child, child2] = [
        scene.entity(&key("child")).unwrap(),
        loaded.entity(&key("child")).unwrap(),
    ];
    assert_eq!(loaded.parent_of(child2), loaded.entity(&key("root")));
    assert_eq!(
        loaded_world.get::<RigidBody>(child2),
        world.get::<RigidBody>(child)
    );
    assert_eq!(
        loaded_world.get::<Follow>(child2),
        world.get::<Follow>(child)
    );
    assert_eq!(
        registry.save_scene(&loaded_world, &loaded),
        text,
        "saving what was loaded writes the same text"
    );
}

#[test]
fn a_scene_key_is_used_once() {
    let (_, mut scene) = built_world();
    let again = SceneEntity::new(key("root"), scene.entities()[1].entity);
    assert_eq!(scene.push(again).unwrap_err().key, key("root"));
}

#[test]
fn describe_lists_every_component_shape_by_name() {
    let shapes = registry().describe();
    let names: Vec<&ComponentName> = shapes.iter().map(|s| &s.name).collect();
    assert_eq!(
        names,
        [&name("transform"), &name("rigid_body"), &name("follow")]
    );
    let body = shapes[1].shape.definition("RigidBody").unwrap();
    assert!(
        matches!(body, DefinitionDoc::Struct { doc: Some(d), .. } if d == "A body the physics moves.")
    );
    let Shape::Struct(transform) = TransformForm::shape() else {
        panic!("a struct")
    };
    let motor = (transform.fields[0].shape)();
    assert!(matches!(motor, Shape::Named(n) if n.name() == "motor"));
    assert!(
        matches!(shapes[0].shape.root, ShapeNode::Def { .. }),
        "{:?}",
        shapes[0].shape.root
    );
}

#[test]
fn the_registry_is_keyed_by_name_and_type() {
    let mut r = registry();
    assert_eq!(r.name_of::<Transform>(), Some(&name("transform")));
    assert_eq!(r.name_of::<Parent>(), None);
    let taken = r.register::<RigidBody>(name("body")).unwrap_err();
    assert_eq!(
        taken,
        RegisterError::TypeTaken {
            type_name: std::any::type_name::<RigidBody>(),
            registered_as: name("rigid_body")
        }
    );
    let taken = r.register_with::<Transform, TransformCodec>(name("follow"));
    assert!(matches!(taken, Err(RegisterError::NameTaken { .. })));
}

#[test]
fn transform_form_converts_both_ways() {
    let t = Transform::from_motor(Motor3::translation(Vec3::new(1.0, 2.0, 3.0)));
    let text = substrate_data::to_text(&TransformForm::from(t));
    let back: TransformForm = substrate_data::from_text(&text).unwrap().value;
    assert_eq!(Transform::from(back).motor, t.motor);
}

#[test]
fn an_entry_of_the_scene_map_can_be_moved_under_another() {
    let (_world, mut scene) = built_world();
    let root = scene.get(&key("root")).unwrap().entity;
    scene.get_mut(&key("child")).unwrap().parent = Some(key("root"));
    let child = scene.get(&key("child")).unwrap().entity;
    assert_eq!(scene.parent_of(child), Some(root));
    assert!(scene.get_mut(&key("absent")).is_none());
}
