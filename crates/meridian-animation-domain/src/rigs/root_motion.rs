//! The component that turns a joint's travel into the entity's travel.

use meridian_ecs::Component;

use crate::JointId;

/// Marks an [`Animator`](super::Animator) whose animations walk the model:
/// the horizontal (x and z) translation of `joint` is taken out of the pose,
/// which leaves the mesh where the entity is, and added to the entity's
/// `Transform` instead, so the entity covers the ground the animation
/// covers. Pick the skeleton's root joint. With `follow_yaw` the joint's
/// turn about the vertical axis is handed over too, so the entity turns as
/// the animation does and the travel is measured in the heading the entity
/// had; otherwise the rotation of the joint stays in the pose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootMotion {
    /// The skeleton (an index into the rig's skeletons) the joint belongs to.
    pub skeleton: usize,
    /// The joint whose travel moves the entity.
    pub joint: JointId,
    /// Whether the joint's turn about y moves the entity as well.
    pub follow_yaw: bool,
}

impl Component for RootMotion {}
