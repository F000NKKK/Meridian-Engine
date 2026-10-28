//! A tree of joints.

use meridian_gac::mat4_mul;

use super::{Joint, JointMatrix, Pose, SkeletonError};

/// The most joints a skeleton may have: what a skinned draw's joint indices
/// and matrix palette are sized for.
pub const MAX_JOINTS: usize = 256;

const IDENTITY: JointMatrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// The joints of one skinned model, parents before their children, and the
/// transform the whole tree hangs from.
#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    joints: Vec<Joint>,
    root: JointMatrix,
}

impl Skeleton {
    /// A skeleton of `joints`, whose roots move with `root` (the transform
    /// of everything above the skeleton in the model, identity for none).
    ///
    /// # Errors
    ///
    /// [`SkeletonError::TooManyJoints`] past [`MAX_JOINTS`], and
    /// [`SkeletonError::ParentNotBefore`] when a joint's parent does not come
    /// before it, which would make the tree order-dependent or cyclic.
    pub fn new(joints: Vec<Joint>, root: JointMatrix) -> Result<Self, SkeletonError> {
        if joints.len() > MAX_JOINTS {
            return Err(SkeletonError::TooManyJoints {
                count: joints.len(),
                max: MAX_JOINTS,
            });
        }
        for (index, joint) in joints.iter().enumerate() {
            if let Some(parent) = joint.parent.filter(|parent| parent.index() >= index) {
                return Err(SkeletonError::ParentNotBefore {
                    joint: index,
                    parent: parent.index(),
                });
            }
        }
        Ok(Self { joints, root })
    }

    /// The joints, parents before their children.
    #[must_use]
    pub fn joints(&self) -> &[Joint] {
        &self.joints
    }

    /// The transform the roots hang from.
    #[must_use]
    pub fn root(&self) -> JointMatrix {
        self.root
    }

    /// The matrix of each joint that takes a bind-pose vertex to where `pose`
    /// puts it: the joint's model-space matrix times its inverse bind matrix.
    /// A joint missing from a short `pose` keeps its rest transform.
    #[must_use]
    pub fn joint_matrices(&self, pose: &Pose) -> Vec<JointMatrix> {
        let mut world: Vec<JointMatrix> = Vec::with_capacity(self.joints.len());
        let mut skin = Vec::with_capacity(self.joints.len());
        for (index, joint) in self.joints.iter().enumerate() {
            let local = pose
                .transforms()
                .get(index)
                .unwrap_or(&joint.rest)
                .to_matrix();
            let above = joint
                .parent
                .and_then(|parent| world.get(parent.index()))
                .copied()
                .unwrap_or(self.root);
            let model = mat4_mul(above, local);
            skin.push(mat4_mul(model, joint.inverse_bind));
            world.push(model);
        }
        skin
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Self {
            joints: Vec::new(),
            root: IDENTITY,
        }
    }
}
