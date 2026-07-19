//! Compute dispatch runtime: device/context, buffers, and the `ComputeKernel` interface domain crates implement against. Knows nothing about physics, GAC, particles or any other algorithm — see docs/dependency-rules.md rule 5.
//!
//! [`ComputeContext::parallel_for`] is the one place `compute-driver` gets
//! reached (rule 5): below `parallel_threshold` it runs sequentially on
//! the calling thread, at or above it delegates to
//! `compute-driver::ComputeDevice::dispatch_parallel`. That threshold is
//! configurable state (via [`ComputeScheduler`]), not a hardcoded
//! constant domain crates read — the point being that "how many items
//! justify parallelizing" is a policy call, tunable per workload, not a
//! fact about the algorithm.
//!
//! `task-core` is a declared dependency (see docs/threading-model.md) but
//! not yet used here: `JobGraph::add_job` requires `'static` closures,
//! and `parallel_for`'s `work` is borrowed from the caller's stack frame
//! — a real signature mismatch, not an oversight. Reusing `task-core` for
//! this needs a scoped/borrowing job API that doesn't exist yet, so this
//! uses `compute-driver`'s own `std::thread::scope`-based dispatch
//! instead of forcing an awkward fit.

use meridian_compute_driver::{ComputeBuffer, ComputeCapabilities, ComputeDevice};

/// Device-visible dispatch dimensions for a `ComputeKernel` invocation.
#[derive(Debug, Clone, Copy, Default)]
pub struct DispatchSize {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl DispatchSize {
    /// A 1D dispatch of `count` work items (`y = z = 1`) — what every
    /// current `ComputeKernel` (batch `Motor3` work) uses.
    pub fn linear(count: u32) -> Self {
        Self { x: count, y: 1, z: 1 }
    }

    pub fn total(&self) -> usize {
        self.x as usize * self.y as usize * self.z as usize
    }
}

/// Everything a `ComputeKernel` dispatch needs: bound buffers, the active
/// `compute-driver` backend, synchronization state. Consumers reach
/// `compute-driver` only through this type — see
/// docs/dependency-rules.md rule 5.
#[derive(Debug, Clone, Copy)]
pub struct ComputeContext {
    device: ComputeDevice,
    parallel_threshold: usize,
}

impl Default for ComputeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeContext {
    pub fn new() -> Self {
        Self { device: ComputeDevice::new(), parallel_threshold: 1024 }
    }

    fn with_parallel_threshold(mut self, threshold: usize) -> Self {
        self.parallel_threshold = threshold;
        self
    }

    pub fn capabilities(&self) -> ComputeCapabilities {
        self.device.capabilities()
    }

    pub fn allocate_buffer(&self, byte_len: usize) -> ComputeBuffer {
        self.device.allocate_buffer(byte_len)
    }

    /// Runs `work(i)` for every `i` in `0..count`. See the module doc for
    /// the sequential/parallel cutoff policy.
    pub fn parallel_for(&self, count: usize, work: impl Fn(usize) + Sync) {
        if count == 0 {
            return;
        }
        if count < self.parallel_threshold {
            for i in 0..count {
                work(i);
            }
            return;
        }
        self.device.dispatch_parallel(count, work);
    }
}

/// A dispatchable unit of compute work. Domain crates (`gac-compute`, and
/// future `particle-compute`/`physics-compute`/`ai-compute`) implement this
/// for their own kernel types; `compute-runtime` knows nothing about what a
/// kernel computes, only how it gets dispatched and on which backend.
pub trait ComputeKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize);
}

/// The entry point consumers use instead of calling `ComputeKernel::dispatch`
/// directly: owns the [`ComputeContext`] (and its sequential/parallel
/// threshold policy) a kernel dispatch runs against.
#[derive(Debug, Clone, Copy)]
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
        Self { context: ComputeContext::new() }
    }

    /// Below `threshold` work items, a dispatch runs sequentially on the
    /// calling thread rather than fanning out across `compute-driver`'s
    /// worker threads.
    pub fn with_parallel_threshold(threshold: usize) -> Self {
        Self { context: ComputeContext::new().with_parallel_threshold(threshold) }
    }

    pub fn context(&self) -> &ComputeContext {
        &self.context
    }

    pub fn run<K: ComputeKernel>(&self, kernel: &K, size: DispatchSize) {
        kernel.dispatch(&self.context, size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingKernel<'a> {
        counter: &'a AtomicUsize,
    }

    impl ComputeKernel for CountingKernel<'_> {
        fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
            context.parallel_for(size.total(), |_| {
                self.counter.fetch_add(1, Ordering::SeqCst);
            });
        }
    }

    #[test]
    fn dispatch_size_linear_and_total() {
        let size = DispatchSize::linear(42);
        assert_eq!(size.x, 42);
        assert_eq!(size.y, 1);
        assert_eq!(size.z, 1);
        assert_eq!(size.total(), 42);
    }

    #[test]
    fn context_capabilities_report_no_gpu_yet() {
        assert!(!ComputeContext::new().capabilities().gpu_compute);
    }

    #[test]
    fn parallel_for_below_threshold_still_visits_every_index() {
        let ctx = ComputeContext::new().with_parallel_threshold(1_000_000); // force sequential path
        let counter = AtomicUsize::new(0);
        ctx.parallel_for(10, |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn parallel_for_above_threshold_still_visits_every_index() {
        let ctx = ComputeContext::new().with_parallel_threshold(1); // force parallel path
        let counter = AtomicUsize::new(0);
        ctx.parallel_for(5_000, |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(counter.load(Ordering::SeqCst), 5_000);
    }

    #[test]
    fn scheduler_run_dispatches_kernel() {
        let counter = AtomicUsize::new(0);
        let kernel = CountingKernel { counter: &counter };
        let scheduler = ComputeScheduler::with_parallel_threshold(1);
        scheduler.run(&kernel, DispatchSize::linear(2_000));
        assert_eq!(counter.load(Ordering::SeqCst), 2_000);
    }
}
