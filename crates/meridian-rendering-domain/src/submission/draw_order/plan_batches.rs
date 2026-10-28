//! Grouping a frame's draws into instanced batches, in drawing order.

use super::{BatchKey, DrawState, OrderedDraw, order_draws};

/// One draw to place: what it needs and how far it is from the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawItem {
    /// Decides which draws may share a call.
    pub key: BatchKey,
    /// Squared distance from the camera to the draw's origin.
    pub distance_squared: f32,
}

/// Draws issued together: one instanced call per batch.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawBatch {
    /// What every member shares.
    pub key: BatchKey,
    /// Indices into the planned items, in submission order.
    pub members: Vec<usize>,
}

/// Groups `items` into batches and orders them for drawing.
///
/// Only [`DrawState::Opaque`] items with equal keys merge: depth testing
/// makes their order irrelevant. Every other item is its own batch, since
/// blended draws must be sorted one by one and on-top draws keep their
/// submission order. Batches are then ordered by [`order_draws`].
#[must_use]
pub fn plan_batches(items: &[DrawItem]) -> Vec<DrawBatch> {
    let mut batches: Vec<(DrawBatch, f32)> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let shared = (item.key.state == DrawState::Opaque)
            .then(|| batches.iter_mut().find(|(batch, _)| batch.key == item.key))
            .flatten();
        if let Some((batch, _)) = shared {
            batch.members.push(index);
        } else {
            let batch = DrawBatch {
                key: item.key,
                members: vec![index],
            };
            batches.push((batch, item.distance_squared));
        }
    }
    let draws: Vec<OrderedDraw> = batches
        .iter()
        .map(|(batch, distance_squared)| OrderedDraw {
            state: batch.key.state,
            distance_squared: *distance_squared,
        })
        .collect();
    let mut slots: Vec<Option<DrawBatch>> =
        batches.into_iter().map(|(batch, _)| Some(batch)).collect();
    order_draws(&draws)
        .into_iter()
        .filter_map(|index| slots[index].take())
        .collect()
}
