//! One animation of one skeleton.

use crate::Clip;

/// What an animation does to one skeleton's joints.
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletonClip {
    /// The skeleton's index in [`Rig::skeletons`](super::Rig::skeletons).
    pub skeleton: usize,
    /// What it does to that skeleton's joints.
    pub clip: Clip,
}
