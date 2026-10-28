//! The stage that resolves transforms along the entity hierarchy.

use meridian_ecs::{World, propagate_transforms};

use crate::{Shared, Stage};

/// Built-in stage that runs [`propagate_transforms`] over a shared [`World`]
/// every frame, so each entity with a `Transform` has a current
/// `GlobalTransform`. Register it after the stages that move entities
/// (physics, gameplay) and before the ones that read world-space motors
/// (rendering, audio).
#[derive(Debug)]
pub struct TransformStage {
    world: Shared<World>,
}

impl TransformStage {
    /// A stage resolving the transforms of `world`.
    #[must_use]
    pub fn new(world: Shared<World>) -> Self {
        Self { world }
    }
}

impl Stage for TransformStage {
    fn run(&mut self) {
        propagate_transforms(&mut self.world.lock());
    }
}
