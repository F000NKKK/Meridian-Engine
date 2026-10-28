//! The joint matrices a skinned renderable is drawn with.

use std::sync::Arc;

/// One matrix per joint of a skeleton for one frame, each taking a bind-pose
/// vertex to where the pose puts it (column-major, as `Skeleton::joint_matrices`
/// of the animation domain returns them). Shared cheaply between the scene
/// and the renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct JointPalette(Arc<[[[f32; 4]; 4]]>);

impl JointPalette {
    /// A palette of `matrices`.
    #[must_use]
    pub fn new(matrices: Vec<[[f32; 4]; 4]>) -> Self {
        Self(matrices.into())
    }

    /// The matrices, one per joint.
    #[must_use]
    pub fn matrices(&self) -> &[[[f32; 4]; 4]] {
        &self.0
    }
}
