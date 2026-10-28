//! A blend across two parameters.

use super::ParamId;

/// How many of the nearest animations take part in a 2D blend.
const NEAREST: usize = 3;

/// Animations laid out on a plane, such as strafe speed against forward
/// speed. The point the parameters name plays the nearest few animations in
/// proportion to the inverse square of their distance, so it plays one alone
/// when it sits on it and fades smoothly to the others as it leaves.
#[derive(Debug, Clone, PartialEq)]
pub struct Blend2d {
    /// The float parameter for the first axis.
    pub x: ParamId,
    /// The float parameter for the second axis.
    pub y: ParamId,
    /// Each animation (an index into the rig's animations) and the point
    /// where it plays alone.
    pub entries: Vec<([f32; 2], usize)>,
}

impl Blend2d {
    /// The share of each animation at the point (`x`, `y`).
    pub(super) fn weights(&self, x: f32, y: f32) -> Vec<(usize, f32)> {
        let mut near: Vec<(usize, f32)> = self
            .entries
            .iter()
            .map(|(at, animation)| {
                let squared = (at[0] - x).powi(2) + (at[1] - y).powi(2);
                (*animation, squared)
            })
            .collect();
        near.sort_by(|a, b| a.1.total_cmp(&b.1));
        near.truncate(NEAREST);
        if let Some(&(animation, squared)) = near.first()
            && squared <= f32::EPSILON
        {
            return vec![(animation, 1.0)];
        }
        near.into_iter()
            .map(|(animation, squared)| (animation, 1.0 / squared))
            .collect()
    }
}
