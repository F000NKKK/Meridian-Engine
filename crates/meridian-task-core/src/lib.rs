//! Job graph scheduler: worker threads, dependency tracking and work stealing for parallel frame execution.

/// A unit of frame work with declared dependencies (see docs/threading-model.md).
#[derive(Debug, Clone, Default)]
pub struct Job {
    pub name: &'static str,
}

/// A dependency-ordered set of jobs for one frame.
#[derive(Debug, Clone, Default)]
pub struct JobGraph {
    pub jobs: Vec<Job>,
}

/// Runs a `JobGraph` across worker threads with work stealing.
#[derive(Debug)]
pub struct Scheduler {
    pub worker_count: usize,
}
