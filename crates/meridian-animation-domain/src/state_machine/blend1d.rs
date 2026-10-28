//! A blend along one parameter.

use super::ParamId;

/// Animations laid out along a number, such as speed: idle at 0, walk at 1,
/// run at 3. A value between two of them plays both, in proportion to how
/// close it is to each; a value past the ends plays the nearest one.
#[derive(Debug, Clone, PartialEq)]
pub struct Blend1d {
    /// The float parameter that picks the mix.
    pub param: ParamId,
    entries: Vec<(f32, usize)>,
}

impl Blend1d {
    /// A blend of `entries`, each an animation (an index into the rig's
    /// animations) at the parameter value where it plays alone. They need not
    /// be in order.
    #[must_use]
    pub fn new(param: ParamId, mut entries: Vec<(f32, usize)>) -> Self {
        entries.sort_by(|a, b| a.0.total_cmp(&b.0));
        Self { param, entries }
    }

    /// The share of each animation at `value`.
    pub(super) fn weights(&self, value: f32) -> Vec<(usize, f32)> {
        let Some(upper) = self.entries.iter().position(|(at, _)| *at >= value) else {
            return self
                .entries
                .last()
                .map(|(_, a)| (*a, 1.0))
                .into_iter()
                .collect();
        };
        if upper == 0 {
            return vec![(self.entries[0].1, 1.0)];
        }
        let ((low_at, low), (high_at, high)) = (self.entries[upper - 1], self.entries[upper]);
        let t = ((value - low_at) / (high_at - low_at)).clamp(0.0, 1.0);
        vec![(low, 1.0 - t), (high, t)]
    }
}
