//! Registering the stages that finish an animated world in the right order.

use meridian_ecs::World;

use crate::{AnimationStage, FrameDelta, Runtime, Shared, StageId, TransformStage};

/// The ids [`add_world_stages`] registered, for stages that must run after
/// them (rendering, audio) or be selected with [`Runtime::tick_only`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldStages {
    /// The animation stage.
    pub animation: StageId,
    /// The transform stage, which depends on the animation stage.
    pub transform: StageId,
}

/// Registers [`AnimationStage`] after `after` (the stages that move or steer
/// entities: gameplay, physics sync), then [`TransformStage`] after the
/// animation stage. The order is the point: root motion changes the
/// `Transform` of an animated entity, so the transform stage must resolve
/// `GlobalTransform`s only afterwards, or a model and its children are drawn
/// a frame behind. Stages that read world-space motors depend on
/// [`WorldStages::transform`].
pub fn add_world_stages(
    runtime: &mut Runtime,
    world: &Shared<World>,
    delta: &Shared<FrameDelta>,
    after: &[StageId],
) -> WorldStages {
    let animation = runtime.add_stage(
        "animation",
        after,
        AnimationStage::new(world.clone(), delta.clone()),
    );
    let transform = runtime.add_stage(
        "transform",
        &[animation],
        TransformStage::new(world.clone()),
    );
    WorldStages {
        animation,
        transform,
    }
}
