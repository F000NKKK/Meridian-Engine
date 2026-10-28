//! How much smaller the binary form of a big scene is.

mod common;

use common::{Motion, RigidBody, key, registry, transform};
use meridian_data_substrate::{DisplayName, NoResolver, Parent, SceneEntity, SceneMap};
use meridian_ecs::{Transform, World};
use meridian_gac::{Motor3, Vec3};

/// A generated scene of `n` entities: a tree ten wide, each with a
/// transform, a body and a name.
fn generated(n: usize) -> (World, SceneMap) {
    let mut world = World::new();
    let mut scene = SceneMap::new(None);
    let mut spawned = Vec::with_capacity(n);
    for i in 0..n {
        let entity = world.spawn();
        let at = Vec3::new(i as f32 * 0.37, (i % 17) as f32 * 1.5, -(i as f32) * 0.125);
        world.insert(entity, Transform::from_motor(Motor3::translation(at)));
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
    let compact = substrate_data::text::to_string(&reg.scene_document(&world, &scene));
    println!(
        "10k entities: pretty text {} bytes, compact text {} bytes, binary {} bytes ({:.1}% of pretty, {:.1}% of compact)",
        text.len(),
        compact.len(),
        bytes.len(),
        100.0 * bytes.len() as f64 / text.len() as f64,
        100.0 * bytes.len() as f64 / compact.len() as f64
    );
    assert!(bytes.len() * 3 < text.len());
    let mut again = World::new();
    let back = reg
        .load_scene_binary(&bytes, &mut again, &mut NoResolver)
        .unwrap();
    assert_eq!(reg.save_scene(&again, &back), text);
    let at = Vec3::new(9_999.0 * 0.37, (9_999 % 17) as f32 * 1.5, -9_999.0 * 0.125);
    assert_eq!(
        transform(&again, &back, "entity_9999"),
        Motor3::translation(at)
    );
}
