//! The local transform of every joint at one moment.

use super::{JointId, JointTransform, Skeleton};

/// One [`JointTransform`] per joint of a skeleton. Starts at the rest pose;
/// sampling a [`Clip`](crate::Clip) overwrites the joints it animates.
#[derive(Debug, Clone, PartialEq)]
pub struct Pose {
    transforms: Vec<JointTransform>,
}

impl Pose {
    /// The pose of `skeleton` before any animation.
    #[must_use]
    pub fn rest(skeleton: &Skeleton) -> Self {
        Self {
            transforms: skeleton.joints().iter().map(|joint| joint.rest).collect(),
        }
    }

    /// The local transforms, in the skeleton's joint order.
    #[must_use]
    pub fn transforms(&self) -> &[JointTransform] {
        &self.transforms
    }

    /// The transform of `joint`, `None` when the pose has no such joint.
    pub fn joint_mut(&mut self, joint: JointId) -> Option<&mut JointTransform> {
        self.transforms.get_mut(joint.index())
    }
}
