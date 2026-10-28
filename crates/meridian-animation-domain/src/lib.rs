//! Skeletal animation vocabulary and math (ADR 047).

mod clips;
mod rigs;
mod skeletons;

pub use clips::{
    Channel, Clip, ClipError, Crossfade, Interpolation, Keys, Playback, Repeat, Track, WeightKeys,
    WeightsChannel,
};
pub use rigs::{
    Animation, AnimationLayer, AnimationPlayer, AnimationTarget, Animator, Rig, SkeletonClip,
};
pub use skeletons::{
    Joint, JointId, JointMatrix, JointTransform, MAX_JOINTS, Pose, Skeleton, SkeletonError,
};
