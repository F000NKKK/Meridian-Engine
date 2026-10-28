//! Mixing several animations of a rig into one pose (ADR 047).

use meridian_animation_domain::Pose;
use meridian_rendering_domain::JointPalette;

use super::rig::ModelRig;

/// One animation taking part in a mix: which one, how far in, and how much
/// of it shows over what came before.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationLayer {
    /// An index into [`ModelRig::animations`]; none is the rest pose.
    pub animation: Option<usize>,
    /// Seconds into the animation.
    pub time: f32,
    /// Zero shows the layers before it, one shows only this one. The first
    /// layer of a mix has nothing before it, so its weight does not matter.
    pub weight: f32,
}

impl ModelRig {
    /// The joint matrices of every skeleton with `layers` mixed in order:
    /// the first is the base and each later one is blended over the result
    /// so far by its weight. A crossfade is two layers, the animation
    /// leaving and the one arriving at the fade's weight.
    #[must_use]
    pub fn palettes_mixed(&self, layers: &[AnimationLayer]) -> Vec<JointPalette> {
        self.skeletons
            .iter()
            .enumerate()
            .map(|(index, skeleton)| {
                let mut mixed = self.pose_of(index, layers.first().copied());
                for layer in layers.iter().skip(1) {
                    mixed.blend(&self.pose_of(index, Some(*layer)), layer.weight);
                }
                JointPalette::new(skeleton.joint_matrices(&mixed))
            })
            .collect()
    }

    /// The morph weights of every part with `layers` mixed as for
    /// [`palettes_mixed`](Self::palettes_mixed).
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

#[cfg(test)]
mod tests {
    use meridian_asset_domain::{
        ModelAsset, ModelChannel, ModelClip, ModelInterpolation, ModelJoint, ModelPart,
        ModelProperty, ModelSkeleton, ModelWeightsChannel, MorphTarget, PartMorph,
    };

    use super::*;

    const IDENTITY: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    /// A clip that holds the joint at `x` and the part's one weight at `weight`.
    fn holding(x: f32, weight: f32) -> ModelClip {
        ModelClip {
            channels: vec![ModelChannel {
                skeleton: 0,
                joint: 0,
                property: ModelProperty::Translation,
                interpolation: ModelInterpolation::Step,
                times: vec![0.0],
                values: vec![x, 0.0, 0.0],
            }],
            weights: vec![ModelWeightsChannel {
                part: 0,
                interpolation: ModelInterpolation::Step,
                times: vec![0.0],
                values: vec![weight],
            }],
        }
    }

    fn rig() -> ModelRig {
        let model = ModelAsset {
            parts: vec![ModelPart {
                morph: Some(PartMorph {
                    targets: vec![MorphTarget::default()],
                    weights: vec![0.0],
                }),
                ..ModelPart::default()
            }],
            skeletons: vec![ModelSkeleton {
                joints: vec![ModelJoint {
                    parent: None,
                    translation: [0.0; 3],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0; 3],
                    inverse_bind: IDENTITY,
                }],
                root: IDENTITY,
            }],
            clips: vec![holding(2.0, 0.0), holding(6.0, 1.0)],
            ..ModelAsset::default()
        };
        ModelRig::of(&model).expect("a valid rig")
    }

    fn layer(animation: usize, weight: f32) -> AnimationLayer {
        AnimationLayer {
            animation: Some(animation),
            time: 0.0,
            weight,
        }
    }

    #[test]
    fn a_later_layer_is_blended_over_the_earlier_ones_by_its_weight() {
        let rig = rig();
        let at = |weight: f32| {
            let layers = [layer(0, 1.0), layer(1, weight)];
            (
                rig.palettes_mixed(&layers)[0].matrices()[0][3][0],
                rig.weights_mixed(&layers)[0][0],
            )
        };
        assert_eq!(at(0.0), (2.0, 0.0));
        assert_eq!(at(0.25), (3.0, 0.25));
        assert_eq!(at(1.0), (6.0, 1.0));
    }

    #[test]
    fn one_layer_is_that_animation_and_no_layers_is_the_rest_pose() {
        let rig = rig();
        assert_eq!(
            rig.palettes_mixed(&[layer(1, 0.3)])[0].matrices()[0][3][0],
            6.0
        );
        assert_eq!(rig.palettes_mixed(&[])[0].matrices()[0][3][0], 0.0);
        assert_eq!(rig.weights_mixed(&[]), [vec![0.0]]);
    }
}
