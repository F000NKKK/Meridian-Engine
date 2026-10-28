//! The stage that plays animations on the world's entities (ADR 048).

use std::collections::HashMap;

use meridian_animation_domain::{AnimationTarget, Animator, JointMatrix};
use meridian_ecs::{Entity, World};
use meridian_rendering_domain::{JointPalette, MorphWeights};

use crate::{FrameDelta, Shared, Stage};

/// What one animator produced this frame: the joint matrices of each
/// skeleton and the morph weights of each part.
type Posed = (Vec<Vec<JointMatrix>>, Vec<Vec<f32>>);

/// Built-in stage that advances every [`Animator`] of a shared [`World`] by
/// the frame's [`FrameDelta`] and hands the result to the entities that
/// carry an [`AnimationTarget`] naming it: each gets a [`JointPalette`] for
/// its skeleton and [`MorphWeights`] for its part, which
/// `extract_scene3d` draws. It changes no `Transform`, so it needs no
/// ordering against physics or transform propagation; register it before
/// whatever extracts the scene and after any stage that starts or switches
/// animations.
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

/// Advances every animator by `seconds` and writes the poses to its targets.
fn animate(world: &mut World, seconds: f32) {
    let posed: HashMap<Entity, Posed> = world
        .query_mut::<Animator>()
        .map(|(entity, animator)| {
            animator.player.advance(&animator.rig, seconds);
            let layers = animator.player.layers();
            let matrices = animator.rig.matrices_mixed(&layers);
            let weights = animator.rig.weights_mixed(&layers);
            (entity, (matrices, weights))
        })
        .collect();
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
