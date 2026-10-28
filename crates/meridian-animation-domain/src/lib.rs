//! Skeletal animation vocabulary and math (ADR 047).

mod clips;
mod skeletons;

pub use clips::{
    Channel, Clip, ClipError, Crossfade, Interpolation, Keys, Playback, Repeat, Track, WeightKeys,
    WeightsChannel,
};
pub use skeletons::{
    Joint, JointId, JointMatrix, JointTransform, MAX_JOINTS, Pose, Skeleton, SkeletonError,
};
