//! Skeletons, joints and poses.

mod error;
mod joint;
mod joint_id;
mod joint_transform;
mod pose;
mod skeleton;

pub use error::SkeletonError;
pub use joint::Joint;
pub use joint_id::JointId;
pub use joint_transform::{JointMatrix, JointTransform};
pub use pose::Pose;
pub use skeleton::{MAX_JOINTS, Skeleton};
