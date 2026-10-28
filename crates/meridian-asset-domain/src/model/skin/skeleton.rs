//! The joint tree of one skin.

use super::ModelJoint;

/// A skin's joints, parents before their children, and the transform the
/// roots hang from.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelSkeleton {
    /// The joints, parents first.
    pub joints: Vec<ModelJoint>,
    /// Column-major transform of everything above the roots in the model
    /// (identity when the roots are scene roots).
    pub root: [[f32; 4]; 4],
}
