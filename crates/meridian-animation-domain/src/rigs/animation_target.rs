//! The component that ties a drawn piece to the animator that moves it.

use meridian_ecs::{Component, Entity};

/// Marks an entity (typically the one with the piece's `MeshRenderer`) as
/// moved by the [`Animator`](super::Animator) on `animator`: it receives the
/// joint matrices of `skeleton` and the morph weights of part `part` each
/// frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationTarget {
    /// The entity holding the animator.
    pub animator: Entity,
    /// The skeleton (an index into the rig's skeletons) that skins the
    /// piece, none for a rigid one.
    pub skeleton: Option<usize>,
    /// The piece's part (an index into the rig's rest weights), whose morph
    /// weights it takes.
    pub part: usize,
}

impl Component for AnimationTarget {}
