//! Why a render graph is invalid.

use crate::GraphResourceId;

/// Why [`RenderGraph::execution_order`](crate::RenderGraph::execution_order) couldn't derive a valid order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderGraphError {
    /// Two passes both declared a write to the same resource — this graph
    /// has no rule for which write a later read should see, so it refuses
    /// to guess rather than silently picking one (e.g. insertion order).
    MultipleWriters {
        /// Resource with conflicting writes.
        resource: GraphResourceId,
        /// Index of the first pass writing the resource.
        first_writer: usize,
        /// Index of the conflicting pass.
        second_writer: usize,
    },
    /// The reads/writes declared form a cycle (A reads what B writes, B
    /// reads what A writes) — no valid execution order exists.
    Cycle,
}

impl core::fmt::Display for RenderGraphError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RenderGraphError::MultipleWriters {
                resource,
                first_writer,
                second_writer,
            } => {
                write!(
                    f,
                    "multiple passes ({first_writer}, {second_writer}) writing resource {resource:?}"
                )
            }
            RenderGraphError::Cycle => write!(f, "render graph has a read/write cycle"),
        }
    }
}

impl std::error::Error for RenderGraphError {}

impl meridian_diagnostics::EngineError for RenderGraphError {}
