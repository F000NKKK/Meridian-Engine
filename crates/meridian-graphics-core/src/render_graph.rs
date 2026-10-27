//! The pass graph and its execution order.

use crate::GraphResourceId;
use crate::RenderGraphError;
use crate::RenderPass;
use std::collections::{HashMap, VecDeque};

/// An automatically-ordered set of render passes for one frame. Passes
/// declare resource reads/writes ([`RenderPass::reading`](crate::RenderPass::reading)/
/// [`RenderPass::writing`](crate::RenderPass::writing)); [`execution_order`](Self::execution_order)
/// derives a valid pass order from those declarations — a producer always
/// runs before every pass that reads what it wrote — the same
/// dependency-declared-not-hand-sequenced idea as `task-core`'s `JobGraph`
/// (Kahn's algorithm), applied to resource conflicts instead of explicit
/// job dependencies. A resource nobody writes (e.g. an externally-supplied
/// swapchain texture) constrains nothing; it's available from the start.
#[derive(Debug, Clone, Default)]
pub struct RenderGraph {
    pub passes: Vec<RenderPass>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a pass, returning its index for use elsewhere (e.g. matching
    /// `execution_order`'s output back to which pass ran).
    pub fn add_pass(&mut self, pass: RenderPass) -> usize {
        self.passes.push(pass);
        self.passes.len() - 1
    }

    /// Derives a valid execution order (pass indices into `self.passes`)
    /// from each pass's declared reads/writes.
    pub fn execution_order(&self) -> Result<Vec<usize>, RenderGraphError> {
        let n = self.passes.len();

        let mut writer: HashMap<GraphResourceId, usize> = HashMap::new();
        for (i, pass) in self.passes.iter().enumerate() {
            for &resource in &pass.writes {
                if let Some(&first_writer) = writer.get(&resource) {
                    return Err(RenderGraphError::MultipleWriters {
                        resource,
                        first_writer,
                        second_writer: i,
                    });
                }
                writer.insert(resource, i);
            }
        }

        let mut indegree = vec![0usize; n];
        let mut dependents: Vec<Vec<usize>> = (0..n).map(|_| Vec::new()).collect();
        for (i, pass) in self.passes.iter().enumerate() {
            for &resource in &pass.reads {
                if let Some(&producer) = writer.get(&resource)
                    && producer != i
                {
                    indegree[i] += 1;
                    dependents[producer].push(i);
                }
            }
        }

        let mut queue: VecDeque<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
        let mut order = Vec::with_capacity(n);
        while let Some(i) = queue.pop_front() {
            order.push(i);
            for &d in &dependents[i] {
                indegree[d] -= 1;
                if indegree[d] == 0 {
                    queue.push_back(d);
                }
            }
        }

        if order.len() != n {
            return Err(RenderGraphError::Cycle);
        }
        Ok(order)
    }
}
