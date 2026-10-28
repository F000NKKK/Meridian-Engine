//! One animated property of one joint.

use super::{ModelInterpolation, ModelProperty};

/// Keyframes for one property of one joint.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelChannel {
    /// Index into [`ModelAsset::skeletons`](crate::ModelAsset::skeletons).
    pub skeleton: usize,
    /// Index of the joint in that skeleton's joints.
    pub joint: usize,
    /// The property the keys move.
    pub property: ModelProperty,
    /// How the keys blend.
    pub interpolation: ModelInterpolation,
    /// Key times in seconds.
    pub times: Vec<f32>,
    /// The values, `property.components()` numbers each, one value per time
    /// (three per time for [`ModelInterpolation::CubicSpline`]: in-tangent,
    /// value, out-tangent).
    pub values: Vec<f32>,
}
