//! Runs kernels against one context.

use std::sync::Arc;

use crate::{ComputeContext, ComputeDevice, ComputeKernel, DispatchSize};

/// The entry point consumers use instead of calling `ComputeKernel::dispatch`
/// directly: owns the [`ComputeContext`](crate::ComputeContext) (and its sequential/parallel
/// threshold policy) a kernel dispatch runs against.
#[derive(Debug, Clone)]
pub struct ComputeScheduler {
    context: ComputeContext,
}

impl ComputeScheduler {
    /// A scheduler over a CPU-only context on `device`.
    #[must_use]
    pub fn new(device: Arc<dyn ComputeDevice>) -> Self {
        Self {
            context: ComputeContext::new(device),
        }
    }

    /// Below `threshold` work items, a dispatch runs sequentially on the
    /// calling thread rather than fanning out across `compute-driver`'s
    /// worker threads.
    #[must_use]
    pub fn with_parallel_threshold(device: Arc<dyn ComputeDevice>, threshold: usize) -> Self {
        Self {
            context: ComputeContext::new(device).with_parallel_threshold(threshold),
        }
    }

    pub fn context(&self) -> &ComputeContext {
        &self.context
    }

    pub fn run<K: ComputeKernel>(&self, kernel: &K, size: DispatchSize) {
        kernel.dispatch(&self.context, size);
    }
}
