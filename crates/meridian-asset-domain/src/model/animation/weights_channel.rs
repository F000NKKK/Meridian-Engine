//! Keyframes for the morph weights of one part.

use super::ModelInterpolation;

/// Keyframes for all the morph weights of one part.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelWeightsChannel {
    /// Index into [`ModelAsset::parts`](crate::ModelAsset::parts).
    pub part: usize,
    /// How the keys blend.
    pub interpolation: ModelInterpolation,
    /// Key times in seconds.
    pub times: Vec<f32>,
    /// The values, one weight per target of the part for each time (three
    /// such sets per time for [`ModelInterpolation::CubicSpline`]: in-tangent,
    /// value, out-tangent).
    pub values: Vec<f32>,
}
