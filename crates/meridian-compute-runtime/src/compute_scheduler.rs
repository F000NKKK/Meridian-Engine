//! Runs kernels against one context.

use crate::{ComputeContext, ComputeKernel, DispatchSize};

/// The entry point consumers use instead of calling `ComputeKernel::dispatch`
/// directly: owns the [`ComputeContext`](crate::ComputeContext) (and its sequential/parallel
/// threshold policy) a kernel dispatch runs against.
#[derive(Debug, Clone)]
pub struct ComputeScheduler {
    context: ComputeContext,
}

impl Default for ComputeScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeScheduler {
    pub fn new() -> Self {
        Self {
            context: ComputeContext::new(),
        }
    }

    /// Below `threshold` work items, a dispatch runs sequentially on the
    /// calling thread rather than fanning out across `compute-driver`'s
    /// worker threads.
    pub fn with_parallel_threshold(threshold: usize) -> Self {
        Self {
            context: ComputeContext::new().with_parallel_threshold(threshold),
        }
    }

    pub fn context(&self) -> &ComputeContext {
        &self.context
    }

    pub fn run<K: ComputeKernel>(&self, kernel: &K, size: DispatchSize) {
        kernel.dispatch(&self.context, size);
    }
}
