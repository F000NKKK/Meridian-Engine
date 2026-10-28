//! One joint of a skeleton.

use super::{JointId, JointMatrix, JointTransform};

/// A joint: where it hangs, how it sits at rest and how the mesh was bound
/// to it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Joint {
    /// The joint it moves with; `None` for a root, which moves with the
    /// skeleton's [`root`](super::Skeleton::root) transform.
    pub parent: Option<JointId>,
    /// The joint's local transform before any animation.
    pub rest: JointTransform,
    /// Takes a mesh vertex from the bind pose into this joint's space, the
    /// inverse of the joint's model-space matrix at bind time.
    pub inverse_bind: JointMatrix,
}
