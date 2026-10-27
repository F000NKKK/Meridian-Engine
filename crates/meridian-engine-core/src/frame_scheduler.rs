//! Runs a job graph on the task scheduler.

use meridian_platform_core::CpuCapabilities;
use meridian_task_core::{JobGraph, Scheduler};

/// Runs one frame's [`JobGraph`] across worker threads — the engine-layer
/// application of `task-core`'s generic scheduler (see
/// docs/threading-model.md). Sized by [`FrameScheduler::default`] to the
/// real detected CPU thread count via `platform-core`, not a hardcoded
/// guess.
#[derive(Debug)]
pub struct FrameScheduler {
    scheduler: Scheduler,
}

impl Default for FrameScheduler {
    fn default() -> Self {
        Self::new(CpuCapabilities::detect().threads)
    }
}

impl FrameScheduler {
    pub fn new(worker_count: usize) -> Self {
        Self {
            scheduler: Scheduler::new(worker_count),
        }
    }

    /// Runs `graph` to completion, blocking until every job has finished.
    pub fn run(&self, graph: JobGraph) {
        self.scheduler.run(graph);
    }
}
