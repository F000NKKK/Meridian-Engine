//! Components and a registry shared by the scene tests.

use meridian_ecs_core::{Component, Transform, World};
use meridian_gac_core::Motor3;
use meridian_scene::{
    ComponentName, ComponentRegistry, EntityKey, EntityKind, SceneMap, TransformCodec,
};
use substrate_data::Schema;
use substrate_data::with::RefTo;

/// How a body moves.
#[derive(Debug, Clone, Copy, PartialEq, Schema)]
pub enum Motion {
    /// Moved by forces.
    #[data(rename = "dynamic")]
    Dynamic,
    /// Never moves.
    #[data(rename = "static")]
    Static,
}

/// A body the physics moves.
#[derive(Debug, Clone, PartialEq, Schema)]
pub struct RigidBody {
    /// How it moves.
    pub motion: Motion,
    /// Kilograms.
    pub mass: f32,
}
impl Component for RigidBody {}

/// Follows another entity of the scene.
#[derive(Debug, Clone, PartialEq, Schema)]
pub struct Follow {
    /// What it follows.
    #[data(with = "RefTo<EntityKind>")]
    pub target: EntityKey,
}
impl Component for Follow {}

pub fn name(s: &str) -> ComponentName {
    ComponentName::new(s).unwrap()
}

pub fn key(s: &str) -> EntityKey {
    EntityKey::new(s).unwrap()
}

pub fn registry() -> ComponentRegistry {
    let mut r = ComponentRegistry::new();
    r.register_with::<Transform, TransformCodec>(name("transform"))
        .unwrap();
    r.register::<RigidBody>(name("rigid_body")).unwrap();
    r.register::<Follow>(name("follow")).unwrap();
    r
}

pub fn transform(world: &World, scene: &SceneMap, k: &str) -> Motor3 {
    world
        .get::<Transform>(scene.entity(&key(k)).unwrap())
        .unwrap()
        .motor
}
