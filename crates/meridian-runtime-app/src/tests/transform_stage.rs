//! Tests of the transform stage.

use std::sync::Arc;

use meridian_ecs::{GlobalTransform, Parent, Transform, World};
use meridian_gac::{Motor3, Vec3};

use crate::testing::InlineExecutor;
use crate::*;

fn translated(x: f32) -> Transform {
    Transform {
        motor: Motor3::translation(Vec3::new(x, 0.0, 0.0)),
    }
}

/// The stage must run after a stage that moves the parent and before one
/// that reads the child's world-space motor, within a single `tick`.
#[test]
fn transform_stage_resolves_what_an_earlier_stage_moved() {
    struct MoveParent(Shared<World>, meridian_ecs::Entity);
    impl Stage for MoveParent {
        fn run(&mut self) {
            if let Some(transform) = self.0.lock().get_mut::<Transform>(self.1) {
                transform.motor = Motor3::translation(Vec3::new(10.0, 0.0, 0.0));
            }
        }
    }

    let world = Shared::new(World::new());
    let (parent, child) = {
        let mut world = world.lock();
        let parent = world.spawn();
        world.insert(parent, translated(1.0));
        let child = world.spawn();
        world.insert(child, translated(2.0));
        world.insert(child, Parent(parent));
        (parent, child)
    };
    let mut runtime = Runtime::new(Arc::new(InlineExecutor));
    let moved = runtime.add_stage("move", &[], MoveParent(world.clone(), parent));
    runtime.add_stage("transforms", &[moved], TransformStage::new(world.clone()));

    runtime.tick();

    let world = world.lock();
    let global = world.get::<GlobalTransform>(child).expect("resolved");
    let at = global.motor.transform_point(Vec3::ZERO);
    assert!((at - Vec3::new(12.0, 0.0, 0.0)).length() < 1e-4, "{at:?}");
}
