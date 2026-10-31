//! Sequential chain of [`DspNode`](super::DspNode) effects.

use super::DspNode;

/// A chain of DSP effect nodes, applied in order.
#[derive(Default)]
pub struct DspGraph {
    nodes: Vec<Box<dyn DspNode>>,
}

impl std::fmt::Debug for DspGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DspGraph")
            .field("node_count", &self.nodes.len())
            .finish()
    }
}

impl DspGraph {
    /// Creates an empty effects chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an effect to the end of the chain.
    pub fn push(&mut self, node: impl DspNode + 'static) {
        self.nodes.push(Box::new(node));
    }

    /// Processes `samples` through every effect in insertion order.
    pub fn process(&mut self, samples: &mut [f32]) {
        for node in &mut self.nodes {
            node.process(samples);
        }
    }
}
