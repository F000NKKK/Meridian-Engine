//! One pass of a render graph.

use crate::GraphResourceId;

/// A node in the render graph: declares its resource reads/writes; the
/// graph derives execution order from that, not from manual sequencing.
#[derive(Debug, Clone, Default)]
pub struct RenderPass {
    pub name: &'static str,
    pub reads: Vec<GraphResourceId>,
    pub writes: Vec<GraphResourceId>,
}

impl RenderPass {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            reads: Vec::new(),
            writes: Vec::new(),
        }
    }

    pub fn reading(mut self, resource: GraphResourceId) -> Self {
        self.reads.push(resource);
        self
    }

    pub fn writing(mut self, resource: GraphResourceId) -> Self {
        self.writes.push(resource);
        self
    }
}
