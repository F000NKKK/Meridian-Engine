//! A resource node of a render graph.

/// Identifies a resource within one [`RenderGraph`](crate::RenderGraph) — assigned by whoever
/// builds the graph (e.g. an index into a per-frame resource table), not
/// `resource`'s persistent `ResourceId`: graph resources are
/// transient and frame-scoped, often not existing (an intermediate render
/// target) until the graph itself creates them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphResourceId(pub u32);
