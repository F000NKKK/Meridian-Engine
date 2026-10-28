//! Derives every written resource's lifetime from an execution order.

use std::collections::HashMap;

use super::{RenderGraph, ResourceLifetime};
use crate::GraphResourceId;

/// The lifetime of each resource some pass writes, in `order`'s positions.
/// A resource nobody writes is external (a swapchain image, a persistent
/// texture) and has no lifetime to plan.
pub(super) fn lifetimes(
    graph: &RenderGraph,
    order: &[usize],
) -> HashMap<GraphResourceId, ResourceLifetime> {
    let mut spans: HashMap<GraphResourceId, ResourceLifetime> = HashMap::new();
    for (position, &pass) in order.iter().enumerate() {
        for &resource in &graph.passes[pass].writes {
            spans.insert(
                resource,
                ResourceLifetime {
                    first: position,
                    last: position,
                },
            );
        }
    }
    for (position, &pass) in order.iter().enumerate() {
        for resource in &graph.passes[pass].reads {
            if let Some(span) = spans.get_mut(resource) {
                span.last = span.last.max(position);
            }
        }
    }
    spans
}
