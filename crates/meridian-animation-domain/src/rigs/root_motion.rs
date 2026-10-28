//! The component that turns a joint's travel into the entity's travel.

use meridian_ecs::Component;

use crate::JointId;

/// Marks an [`Animator`](super::Animator) whose animations walk the model:
/// the horizontal (x and z) translation of `joint` is taken out of the pose,
/// which leaves the mesh where the entity is, and added to the entity's
/// `Transform` instead, so the entity covers the ground the animation
/// covers. Pick the skeleton's root joint. Rotation of the joint stays in the
/// pose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootMotion {
    /// The skeleton (an index into the rig's skeletons) the joint belongs to.
    pub skeleton: usize,
    /// The joint whose travel moves the entity.
    pub joint: JointId,
}

impl Component for RootMotion {}
