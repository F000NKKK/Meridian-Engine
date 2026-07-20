//! Sequential chain of [`DspNode`](super::DspNode) effects.

use super::DspNode;

/// A chain of DSP effect nodes, applied in order.
#[derive(Default)]
pub struct DspGraph {
    nodes: Vec<Box<dyn DspNode>>,
}

impl DspGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, node: impl DspNode + 'static) {
        self.nodes.push(Box::new(node));
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        for node in &mut self.nodes {
            node.process(samples);
        }
    }
}
