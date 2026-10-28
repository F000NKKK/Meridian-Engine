//! How long a transient resource is live within one frame.

/// The span of execution positions (indices into
/// [`RenderGraph::execution_order`](crate::RenderGraph::execution_order))
/// over which a transient resource must exist: from the pass that writes it
/// to the last pass that reads it, both inclusive. A resource nobody reads
/// lives for its writer's position only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceLifetime {
    /// Position of the pass that writes the resource.
    pub first: usize,
    /// Position of the last pass that reads it (the writer's if none does).
    pub last: usize,
}

impl ResourceLifetime {
    /// Whether `self` and `other` share an execution position. A pass reads
    /// its inputs and writes its outputs at the same position, so two
    /// resources touching at one pass cannot share memory.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self.first <= other.last && other.first <= self.last
    }
}
