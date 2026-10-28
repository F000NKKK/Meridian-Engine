//! The skeletons and animations of a loaded model (ADR 047).

use meridian_animation_domain::{
    Channel, Clip, Interpolation, JointId, JointTransform, Keys, Skeleton, Track, WeightKeys,
    WeightsChannel,
};
pub use meridian_animation_domain::{Playback, Repeat as AnimationRepeat};
use meridian_asset_domain::{
    ModelAsset, ModelChannel, ModelClip, ModelInterpolation, ModelProperty, ModelSkeleton,
    ModelWeightsChannel,
};
use meridian_gac::{Rotor, Vec3};
use meridian_rendering_domain::JointPalette;

use super::mixing::AnimationLayer;

/// One animation of a skeleton.
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletonClip {
    /// The skeleton's index in [`ModelRig::skeletons`].
    pub skeleton: usize,
    /// What it does to that skeleton's joints.
    pub clip: Clip,
}

/// A glTF animation: the clips of the skeletons it moves, which play together.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelAnimation {
    /// One clip per skeleton the animation moves.
    pub clips: Vec<SkeletonClip>,
    /// What it does to the morph weights of the model's parts.
    pub weights: Vec<WeightsChannel>,
}

impl ModelAnimation {
    /// How long the animation runs: its longest clip or weight track.
    #[must_use]
    pub fn duration(&self) -> f32 {
        let clips = self.clips.iter().map(|entry| entry.clip.duration());
        let weights = self.weights.iter().map(|channel| channel.keys.end_time());
        clips.chain(weights).fold(0.0, f32::max)
    }
}

/// The skeletons of a model and the animations that move them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelRig {
    /// One skeleton per skin of the file.
    pub skeletons: Vec<Skeleton>,
    /// The file's animations of joints and morph weights.
    pub animations: Vec<ModelAnimation>,
    /// Per part, the morph weights before any animation; empty for a part
    /// without blend shapes.
    pub rest_weights: Vec<Vec<f32>>,
}

impl ModelRig {
    /// The rig of `model`.
    ///
    /// # Errors
    ///
    /// A skeleton or animation the animation domain refuses (too many
    /// joints, malformed keys), as text.
    pub(crate) fn of(model: &ModelAsset) -> Result<Self, String> {
        let skeletons = model
            .skeletons
            .iter()
            .map(skeleton)
            .collect::<Result<Vec<_>, _>>()?;
        let animations = model
            .clips
            .iter()
            .map(|clip| animation(clip, model))
            .collect::<Result<Vec<_>, _>>()?;
        let rest_weights = model
            .parts
            .iter()
            .map(|part| part.morph.as_ref().map(|morph| morph.weights.clone()))
            .map(Option::unwrap_or_default)
            .collect();
        Ok(Self {
            skeletons,
            animations,
            rest_weights,
        })
    }

    /// The morph weights of every part with `animation` (as for
    /// [`palettes`](Self::palettes)) sampled at `time`; a part without blend
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

    /// The joint matrices of every skeleton with `animation` (an index into
    /// [`animations`](Self::animations); none leaves the rest pose) sampled
    /// at `time`.
    #[must_use]
    pub fn palettes(&self, animation: Option<usize>, time: f32) -> Vec<JointPalette> {
        self.palettes_mixed(&[AnimationLayer {
            animation,
            time,
            weight: 1.0,
        }])
    }
}

fn skeleton(source: &ModelSkeleton) -> Result<Skeleton, String> {
    let joints = source
        .joints
        .iter()
        .map(|joint| {
            Ok(meridian_animation_domain::Joint {
                parent: match joint.parent {
                    Some(parent) => Some(joint_id(parent)?),
                    None => None,
                },
                rest: JointTransform {
                    translation: Vec3::new(
                        joint.translation[0],
                        joint.translation[1],
                        joint.translation[2],
                    ),
                    rotation: Rotor::from_quaternion(joint.rotation),
                    scale: Vec3::new(joint.scale[0], joint.scale[1], joint.scale[2]),
                },
                inverse_bind: joint.inverse_bind,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Skeleton::new(joints, source.root).map_err(|err| err.to_string())
}

fn joint_id(index: usize) -> Result<JointId, String> {
    JointId::from_index(index).ok_or_else(|| format!("joint {index} is out of range"))
}

fn animation(source: &ModelClip, model: &ModelAsset) -> Result<ModelAnimation, String> {
    let mut clips = Vec::new();
    for skeleton in 0..model.skeletons.len() {
        let channels = source
            .channels
            .iter()
            .filter(|channel| channel.skeleton == skeleton)
            .map(channel)
            .collect::<Result<Vec<_>, _>>()?;
        if !channels.is_empty() {
            clips.push(SkeletonClip {
                skeleton,
                clip: Clip::new(channels),
            });
        }
    }
    let weights = source
        .weights
        .iter()
        .map(|channel| weights_channel(channel, model))
        .collect::<Result<_, _>>()?;
    Ok(ModelAnimation { clips, weights })
}

fn weights_channel(
    source: &ModelWeightsChannel,
    model: &ModelAsset,
) -> Result<WeightsChannel, String> {
    let targets = model
        .parts
        .get(source.part)
        .and_then(|part| part.morph.as_ref())
        .map_or(0, |morph| morph.targets.len());
    let keys = WeightKeys::new(
        &source.times,
        &source.values,
        targets,
        interpolation(source.interpolation),
    )
    .map_err(|err| err.to_string())?;
    Ok(WeightsChannel {
        target: source.part,
        keys,
    })
}

fn interpolation(source: ModelInterpolation) -> Interpolation {
    match source {
        ModelInterpolation::Linear => Interpolation::Linear,
        ModelInterpolation::Step => Interpolation::Step,
        ModelInterpolation::CubicSpline => Interpolation::CubicSpline,
    }
}

fn channel(source: &ModelChannel) -> Result<Channel, String> {
    let interpolation = interpolation(source.interpolation);
    let track = match source.property {
        ModelProperty::Translation => Track::Translation(keys(source, interpolation)?),
        ModelProperty::Rotation => Track::Rotation(keys(source, interpolation)?),
        ModelProperty::Scale => Track::Scale(keys(source, interpolation)?),
    };
    Ok(Channel {
        target: joint_id(source.joint)?,
        track,
    })
}

/// The channel's flat values grouped `N` to a value.
fn keys<const N: usize>(
    source: &ModelChannel,
    interpolation: Interpolation,
) -> Result<Keys<N>, String> {
    let values = source
        .values
        .chunks_exact(N)
        .map(|chunk| std::array::from_fn(|i| chunk[i]))
        .collect();
    Keys::new(source.times.clone(), values, interpolation).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use meridian_asset_domain::{ModelJoint, ModelSkeleton};

    use super::*;

    const IDENTITY: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    fn joint(parent: Option<usize>, translation: [f32; 3]) -> ModelJoint {
        ModelJoint {
            parent,
            translation,
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            inverse_bind: IDENTITY,
        }
    }

    /// A root and a child one unit above it, and a clip that slides the child
    /// from one unit to three units above over two seconds.
    fn model() -> ModelAsset {
        ModelAsset {
            skeletons: vec![ModelSkeleton {
                joints: vec![joint(None, [0.0; 3]), joint(Some(0), [0.0, 1.0, 0.0])],
                root: IDENTITY,
            }],
            clips: vec![ModelClip {
                channels: vec![ModelChannel {
                    skeleton: 0,
                    joint: 1,
                    property: ModelProperty::Translation,
                    interpolation: ModelInterpolation::Linear,
                    times: vec![0.0, 2.0],
                    values: vec![0.0, 1.0, 0.0, 0.0, 3.0, 0.0],
                }],
                weights: Vec::new(),
            }],
            ..ModelAsset::default()
        }
    }

    #[test]
    fn the_rest_palette_places_each_joint_where_it_sits_at_rest() {
        let rig = ModelRig::of(&model()).expect("a valid rig");
        let palettes = rig.palettes(None, 0.0);
        assert_eq!(palettes[0].matrices()[1][3], [0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn an_animation_moves_its_joint_along_its_keys() {
        let rig = ModelRig::of(&model()).expect("a valid rig");
        assert_eq!(rig.animations[0].duration(), 2.0);
        let palettes = rig.palettes(Some(0), 1.0);
        assert_eq!(palettes[0].matrices()[1][3], [0.0, 2.0, 0.0, 1.0]);
        assert_eq!(palettes[0].matrices()[0][3], [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn an_unknown_animation_leaves_the_rest_pose() {
        let rig = ModelRig::of(&model()).expect("a valid rig");
        let palettes = rig.palettes(Some(9), 1.0);
        assert_eq!(palettes[0].matrices()[1][3], [0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn keys_the_animation_domain_refuses_are_an_error() {
        let mut broken = model();
        broken.clips[0].channels[0].times = vec![2.0, 0.0];
        assert!(ModelRig::of(&broken).is_err());
    }

    #[test]
    fn morph_weights_start_at_rest_and_follow_the_animation() {
        use meridian_asset_domain::{ModelPart, ModelWeightsChannel, MorphTarget, PartMorph};
        let morph = PartMorph {
            targets: vec![MorphTarget::default(); 2],
            weights: vec![0.25, 0.0],
        };
        let model = ModelAsset {
            parts: vec![
                ModelPart::default(),
                ModelPart {
                    morph: Some(morph),
                    ..ModelPart::default()
                },
            ],
            clips: vec![ModelClip {
                weights: vec![ModelWeightsChannel {
                    part: 1,
                    interpolation: ModelInterpolation::Linear,
                    times: vec![0.0, 2.0],
                    values: vec![0.0, 1.0, 1.0, 0.0],
                }],
                ..ModelClip::default()
            }],
            ..ModelAsset::default()
        };
        let rig = ModelRig::of(&model).expect("a valid rig");
        assert_eq!(rig.weights(None, 1.0), [vec![], vec![0.25, 0.0]]);
        assert_eq!(rig.weights(Some(0), 1.0), [vec![], vec![0.5, 0.5]]);
        assert!((rig.animations[0].duration() - 2.0).abs() < f32::EPSILON);
    }
}
