//! Skeletal animation vocabulary and math (ADR 047).

mod clips;
mod skeletons;

pub use clips::{Channel, Clip, ClipError, Interpolation, Keys, Playback, Repeat, Track};
pub use skeletons::{
    Joint, JointId, JointMatrix, JointTransform, MAX_JOINTS, Pose, Skeleton, SkeletonError,
};
