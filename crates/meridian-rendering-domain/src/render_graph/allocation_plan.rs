//! Which transient resources of a frame share one allocation.

use std::collections::HashMap;
use std::hash::Hash;

use super::{RenderGraph, ResourceLifetime};
use crate::{GraphResourceId, RenderGraphError};

/// Maps each transient resource to a slot; resources in one slot never live
/// at the same time and have the same compatibility key, so a backend
/// allocates `slot_count` targets instead of one per resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationPlan {
    slots: HashMap<GraphResourceId, usize>,
    slot_count: usize,
}

impl AllocationPlan {
    /// The slot `resource` was assigned, or `None` if it is not transient.
    #[must_use]
    pub fn slot(&self, resource: GraphResourceId) -> Option<usize> {
        self.slots.get(&resource).copied()
    }

    /// How many distinct allocations the frame needs.
    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.slot_count
    }
}

impl RenderGraph {
    /// Each resource some pass writes, with its lifetime in execution
    /// positions (see [`ResourceLifetime`]).
    ///
    /// # Errors
    ///
    /// Returns the [`RenderGraphError`] of
    /// [`execution_order`](Self::execution_order) when the graph is invalid.
    pub fn resource_lifetimes(
        &self,
    ) -> Result<HashMap<GraphResourceId, ResourceLifetime>, RenderGraphError> {
        let order = self.execution_order()?;
        Ok(super::lifetimes::lifetimes(self, &order))
    }

    /// Assigns the resources in `keys` to shared slots. `keys` holds only
    /// the transient resources the backend allocates (a resource absent
    /// from it is external and untouched); two resources may share a slot
    /// only when their keys are equal (a backend passes whatever makes two
    /// targets interchangeable: size, format, usage) and their lifetimes do
    /// not overlap.
    ///
    /// # Errors
    ///
    /// Returns the [`RenderGraphError`] of
    /// [`execution_order`](Self::execution_order) when the graph is invalid.
    pub fn plan_allocations<K: Eq + Hash>(
        &self,
        keys: &HashMap<GraphResourceId, K>,
    ) -> Result<AllocationPlan, RenderGraphError> {
        let spans = self.resource_lifetimes()?;
        let mut transient: Vec<(GraphResourceId, ResourceLifetime)> = spans
            .into_iter()
            .filter(|(resource, _)| keys.contains_key(resource))
            .collect();
        transient.sort_by_key(|(resource, span)| (span.first, resource.0));

        let mut slots = HashMap::new();
        // Per slot: its key and the last position its latest occupant uses.
        let mut busy_until: Vec<(&K, usize)> = Vec::new();
        for (resource, span) in transient {
            let key = &keys[&resource];
            let reusable = busy_until
                .iter()
                .position(|&(slot_key, last)| slot_key == key && last < span.first);
            let slot = reusable.unwrap_or_else(|| {
                busy_until.push((key, 0));
                busy_until.len() - 1
            });
            busy_until[slot] = (key, span.last);
            slots.insert(resource, slot);
        }
        Ok(AllocationPlan {
            slots,
            slot_count: busy_until.len(),
        })
    }
}
