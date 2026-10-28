//! The stage that plays animations on the world's entities (ADR 048).

use std::collections::HashMap;

use meridian_animation_domain::{
    AnimationController, AnimationTarget, Animator, JointMatrix, RootDelta, RootMotion,
};
use meridian_ecs::{Entity, Transform, World};
use meridian_gac::{Motor3, Vec3};
use meridian_rendering_domain::{JointPalette, MorphWeights};

use crate::{FrameDelta, Shared, Stage};

/// What one animator produced this frame: the joint matrices of each
/// skeleton and the morph weights of each part.
type Posed = (Vec<Vec<JointMatrix>>, Vec<Vec<f32>>);

/// Built-in stage that advances every [`Animator`] of a shared [`World`] by
/// the frame's [`FrameDelta`] and hands the result to the entities that
/// carry an [`AnimationTarget`] naming it: each gets a [`JointPalette`] for
/// its skeleton and [`MorphWeights`] for its part, which
/// `extract_scene3d` draws. It changes no `Transform` except the one of an
/// animator with [`RootMotion`], which it moves by the ground the animation
/// covered, so register it before the transform stage that resolves
/// `GlobalTransform`s and keep physics off such an entity. An animator with an
/// [`AnimationController`] is steered by it first, so gameplay switches
/// animations by setting the controller's parameters. Register the stage
/// before whatever extracts the scene and after any stage that sets them.
#[derive(Debug)]
pub struct AnimationStage {
    world: Shared<World>,
    delta: Shared<FrameDelta>,
}

impl AnimationStage {
    /// A stage playing the animators of `world` by the time `delta` holds.
    #[must_use]
    pub fn new(world: Shared<World>, delta: Shared<FrameDelta>) -> Self {
        Self { world, delta }
    }
}

impl Stage for AnimationStage {
    fn run(&mut self) {
        let seconds = self.delta.lock().0;
        animate(&mut self.world.lock(), seconds);
    }
}

/// Lets every controller steer its animator: transitions fire before the
/// player moves on, so a new state starts this frame.
fn steer(world: &mut World) {
    let steered: Vec<Entity> = world
        .query::<AnimationController>()
        .map(|(entity, _)| entity)
        .collect();
    for entity in steered {
        let Some(mut controller) = world.get::<AnimationController>(entity).cloned() else {
            continue;
        };
        if let Some(animator) = world.get_mut::<Animator>(entity) {
            let rig = animator.rig.clone();
            controller.update(&mut animator.player, &rig);
            world.insert(entity, controller);
        }
    }
}

/// Moves `entity` by `travel` in its own frame: the displacement was
/// measured in the heading the entity had before the step, so it is applied
/// in that frame, and the entity's turn comes with it.
fn walk(world: &mut World, entity: Entity, travel: RootDelta) {
    if let Some(transform) = world.get_mut::<Transform>(entity) {
        let turn = Motor3::rotation(Vec3::new(0.0, 1.0, 0.0), travel.yaw);
        let step = Motor3::translation(travel.translation);
        transform.motor = turn.compose(step).compose(transform.motor);
    }
}

/// Advances every animator by `seconds` and writes the poses to its targets.
fn animate(world: &mut World, seconds: f32) {
    steer(world);
    let animators: Vec<Entity> = world
        .query::<Animator>()
        .map(|(entity, _)| entity)
        .collect();
    let mut posed: HashMap<Entity, Posed> = HashMap::new();
    for entity in animators {
        let pin = world.get::<RootMotion>(entity).copied();
        let Some(animator) = world.get_mut::<Animator>(entity) else {
            continue;
        };
        let travel = if let Some(pin) = pin {
            animator.player.advance_travel(&animator.rig, seconds, pin)
        } else {
            animator.player.advance(&animator.rig, seconds);
            RootDelta::default()
        };
        let layers = animator.player.layers();
        let matrices = animator.rig.matrices_pinned(&layers, pin);
        let weights = animator.rig.weights_mixed(&layers);
        posed.insert(entity, (matrices, weights));
        if pin.is_some() {
            walk(world, entity, travel);
        }
    }
    let targets: Vec<(Entity, AnimationTarget)> = world
        .query::<AnimationTarget>()
        .map(|(entity, target)| (entity, *target))
        .collect();
    for (entity, target) in targets {
        let Some((matrices, weights)) = posed.get(&target.animator) else {
            continue;
        };
        let palette = target
            .skeleton
            .and_then(|skeleton| matrices.get(skeleton))
            .filter(|joints| !joints.is_empty());
        if let Some(joints) = palette {
            world.insert(entity, JointPalette::new(joints.clone()));
        }
        if let Some(part) = weights.get(target.part).filter(|part| !part.is_empty()) {
            world.insert(entity, MorphWeights(part.clone()));
        }
    }
}
