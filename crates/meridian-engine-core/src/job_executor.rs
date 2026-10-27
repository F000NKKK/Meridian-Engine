//! The port that runs a frame's jobs.

use crate::Job;

/// Runs a dependency-ordered set of jobs, independent ones in parallel if
/// the implementation has workers. The runtime decides *what* runs and in
/// which order; an executor only supplies the threads
/// (`meridian-task-core` is the real one).
///
/// Every implementation must pass
/// [`testing::run_executor_contract`](crate::testing).
pub trait JobExecutor: Send + Sync {
    /// Runs every job's action exactly once, a job only after all of its
    /// `deps` have finished, and returns once all are done.
    fn execute(&self, jobs: Vec<Job>);
}
