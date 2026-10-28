//! The order a frame's draws are issued in.

use super::DrawState;

/// What ordering needs to know about one draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderedDraw {
    /// The draw's composite state.
    pub state: DrawState,
    /// Squared distance from the camera to the draw's origin.
    pub distance_squared: f32,
}

/// The indices of `draws` in the order to issue them: grouped by
/// [`DrawState`] in its declaration order; within a blended state farthest
/// first (so nearer translucent surfaces blend over farther ones), within
/// the others in submission order. Stable, so equal distances keep their
/// submission order.
#[must_use]
pub fn order_draws(draws: &[OrderedDraw]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..draws.len()).collect();
    order.sort_by(|&a, &b| {
        let (left, right) = (draws[a], draws[b]);
        left.state.cmp(&right.state).then_with(|| {
            if left.state.blends() {
                right.distance_squared.total_cmp(&left.distance_squared)
            } else {
                core::cmp::Ordering::Equal
            }
        })
    });
    order
}
