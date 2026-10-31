//! Runtime stage resolving independent spatial components.

use meridian_ecs::World;

use crate::{Shared, SpatialIssue, Stage, resolve_spatial_components};

/// Register after source edits and before `TransformStage`. Inspect issues after run.
/// Collision/render components remain on the same entity and are not replaced.
#[derive(Debug)]
pub struct SpatialStage {
    world: Shared<World>,
    issues: Shared<Vec<SpatialIssue>>,
}

impl SpatialStage {
    /// Resolves the independent spatial sources in this world.
    #[must_use]
    pub fn new(world: Shared<World>) -> Self {
        Self {
            world,
            issues: Shared::new(Vec::new()),
        }
    }

    /// Invalid entities from the most recent run; no issue is silently discarded.
    #[must_use]
    pub fn issues(&self) -> Shared<Vec<SpatialIssue>> {
        self.issues.clone()
    }
}

impl Stage for SpatialStage {
    fn run(&mut self) {
        let issues = resolve_spatial_components(&mut self.world.lock());
        *self.issues.lock() = issues;
    }
}
