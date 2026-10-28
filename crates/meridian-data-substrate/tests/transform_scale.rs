//! `scale` in a saved transform: round trip, and scenes written before it.

mod common;

use common::{key, registry};
use meridian_data_substrate::NoResolver;
use meridian_ecs::{Transform, World};
use meridian_gac::{Motor3, Vec3};

const BEFORE_SCALE: &str = r#"{
  format: 1,
  entities: [
    { id: "old", components: { transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 2, 0] } } } },
  ],
}
"#;

const WITH_SCALE: &str = r#"{
  format: 1,
  entities: [
    { id: "big", components: { transform: { scale: [2, 3, 4] } } },
  ],
}
"#;

fn load(text: &str) -> (World, meridian_data_substrate::SceneMap) {
    let mut world = World::new();
    let scene = registry()
        .load_scene(text, &mut world, &mut NoResolver)
        .unwrap();
    (world, scene)
}

#[test]
fn a_scene_written_before_scale_loads_at_unit_scale() {
    let (world, scene) = load(BEFORE_SCALE);
    let transform = world
        .get::<Transform>(scene.entity(&key("old")).unwrap())
        .unwrap();
    assert_eq!(transform.scale, Vec3::ONE);
    assert_eq!(
        transform.motor.transform_point(Vec3::ZERO),
        Vec3::new(0.0, 2.0, 0.0)
    );
}

#[test]
fn a_scale_alone_leaves_the_motor_at_identity() {
    let (world, scene) = load(WITH_SCALE);
    let transform = world
        .get::<Transform>(scene.entity(&key("big")).unwrap())
        .unwrap();
    assert_eq!(transform.scale, Vec3::new(2.0, 3.0, 4.0));
    assert_eq!(common::transform(&world, &scene, "big"), Motor3::identity());
}

#[test]
fn scale_survives_a_save() {
    let (world, scene) = load(WITH_SCALE);
    let text = registry().save_scene(&world, &scene);
    let (again, again_scene) = load(&text);
    let transform = again
        .get::<Transform>(again_scene.entity(&key("big")).unwrap())
        .unwrap();
    assert_eq!(transform.scale, Vec3::new(2.0, 3.0, 4.0));
}
