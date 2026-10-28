//! The skeletons of a model and the animations that move them.

use super::{Animation, AnimationLayer};
use crate::{JointMatrix, Pose, Skeleton};

/// The skeletons of a model and the animations that move them and its morph
/// weights (ADR 047). Plain data, built once when a model loads and shared
/// by everything that plays it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rig {
    /// One skeleton per skin of the file.
    pub skeletons: Vec<Skeleton>,
    /// The file's animations of joints and morph weights.
    pub animations: Vec<Animation>,
    /// Per part, the morph weights before any animation; empty for a part
    /// without blend shapes.
    pub rest_weights: Vec<Vec<f32>>,
}

impl Rig {
    /// The joint matrices of every skeleton with `animation` (an index into
    /// [`animations`](Self::animations); none leaves the rest pose) sampled
    /// at `time`.
    #[must_use]
    pub fn matrices(&self, animation: Option<usize>, time: f32) -> Vec<Vec<JointMatrix>> {
        self.matrices_mixed(&[AnimationLayer {
            animation,
            time,
            weight: 1.0,
        }])
    }

    /// The joint matrices of every skeleton with `layers` mixed in order:
    /// the first is the base and each later one is blended over the result
    /// so far by its weight. A crossfade is two layers, the animation
    /// leaving and the one arriving at the fade's weight.
    #[must_use]
    pub fn matrices_mixed(&self, layers: &[AnimationLayer]) -> Vec<Vec<JointMatrix>> {
        self.skeletons
            .iter()
            .enumerate()
            .map(|(index, skeleton)| {
                let mut mixed = self.pose_of(index, layers.first().copied());
                for layer in layers.iter().skip(1) {
                    mixed.blend(&self.pose_of(index, Some(*layer)), layer.weight);
                }
                skeleton.joint_matrices(&mixed)
            })
            .collect()
    }

    /// The morph weights of every part with `animation` (as for
    /// [`matrices`](Self::matrices)) sampled at `time`; a part without blend
    /// shapes has none.
    #[must_use]
    pub fn weights(&self, animation: Option<usize>, time: f32) -> Vec<Vec<f32>> {
        let channels = animation
            .and_then(|index| self.animations.get(index))
            .map(|animation| animation.weights.as_slice())
            .unwrap_or_default();
        let mut weights = self.rest_weights.clone();
        for channel in channels {
            if let Some(part) = weights.get_mut(channel.target) {
                channel.keys.sample(time, part);
            }
        }
        weights
    }

    /// The morph weights of every part with `layers` mixed as for
    /// [`matrices_mixed`](Self::matrices_mixed).
    #[must_use]
    pub fn weights_mixed(&self, layers: &[AnimationLayer]) -> Vec<Vec<f32>> {
        let at = |layer: &AnimationLayer| self.weights(layer.animation, layer.time);
        let mut mixed = layers.first().map_or_else(|| self.weights(None, 0.0), at);
        for layer in layers.iter().skip(1) {
            let next = at(layer);
            for (part, target) in mixed.iter_mut().zip(next) {
                for (weight, goal) in part.iter_mut().zip(target) {
                    *weight += (goal - *weight) * layer.weight;
                }
            }
        }
        mixed
    }

    /// The pose of skeleton `skeleton` under `layer` alone.
    fn pose_of(&self, skeleton: usize, layer: Option<AnimationLayer>) -> Pose {
        let mut pose = Pose::rest(&self.skeletons[skeleton]);
        let clips = layer
            .and_then(|layer| self.animations.get(layer.animation?))
            .map(|animation| animation.clips.as_slice())
            .unwrap_or_default();
        for entry in clips.iter().filter(|entry| entry.skeleton == skeleton) {
            entry
                .clip
                .sample(layer.map_or(0.0, |layer| layer.time), &mut pose);
        }
        pose
    }
}
