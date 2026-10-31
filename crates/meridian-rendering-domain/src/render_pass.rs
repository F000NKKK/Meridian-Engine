//! One pass of a render graph.

use crate::GraphResourceId;

/// A node in the render graph: declares its resource reads/writes; the
/// graph derives execution order from that, not from manual sequencing.
#[derive(Debug, Clone, Default)]
pub struct RenderPass {
    /// Human-readable pass name used in diagnostics.
    pub name: &'static str,
    /// Resources this pass reads.
    pub reads: Vec<GraphResourceId>,
    /// Resources this pass writes.
    pub writes: Vec<GraphResourceId>,
}

impl RenderPass {
    /// Creates a pass with no declared resource dependencies.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            reads: Vec::new(),
            writes: Vec::new(),
        }
    }

    /// Declares a resource read and returns the updated pass.
    pub fn reading(mut self, resource: GraphResourceId) -> Self {
        self.reads.push(resource);
        self
    }

    /// Declares a resource write and returns the updated pass.
    pub fn writing(mut self, resource: GraphResourceId) -> Self {
        self.writes.push(resource);
        self
    }
}
