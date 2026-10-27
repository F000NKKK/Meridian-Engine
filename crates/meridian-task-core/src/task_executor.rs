//! The runtime's job-executor port on this crate's scheduler.

use meridian_engine_core::{Job, JobExecutor};

use crate::{JobGraph, JobId, Scheduler};

/// Implements `engine-core`'s [`JobExecutor`] port with a [`Scheduler`]: the
/// jobs become a [`JobGraph`] and run across the worker threads, independent
/// branches in parallel.
#[derive(Debug)]
pub struct TaskExecutor {
    scheduler: Scheduler,
}

impl TaskExecutor {
    /// An executor with `worker_count` workers (at least 1); the composition
    /// root sizes it, e.g. from the detected CPU thread count.
    #[must_use]
    pub fn new(worker_count: usize) -> Self {
        Self {
            scheduler: Scheduler::new(worker_count),
        }
    }
}

impl JobExecutor for TaskExecutor {
    fn execute(&self, jobs: Vec<Job>) {
        let mut graph = JobGraph::new();
        let mut ids: Vec<JobId> = Vec::with_capacity(jobs.len());
        for job in jobs {
            let deps: Vec<JobId> = job.deps.iter().map(|&index| ids[index]).collect();
            ids.push(graph.add_job(job.name, &deps, job.action));
        }
        self.scheduler.run(graph);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use meridian_engine_core::testing::run_executor_contract;
    use meridian_engine_core::{Runtime, Shared, Stage};

    use super::*;

    #[test]
    fn task_executor_passes_the_executor_contract() {
        run_executor_contract(&TaskExecutor::new(4));
    }

    /// Two stages with no dependency, each locking a *different* resource,
    /// must both complete without blocking on each other — the guarantee
    /// per-stage state handles exist for. Repeated to give a real deadlock
    /// a chance to show.
    #[test]
    fn independent_stages_on_different_resources_never_deadlock() {
        struct Touch<T>(Shared<T>, Arc<AtomicUsize>);
        impl<T: Send> Stage for Touch<T> {
            fn run(&mut self) {
                let _guard = self.0.lock();
                self.1.fetch_add(1, Ordering::SeqCst);
            }
        }

        let (first_runs, second_runs) =
            (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let mut runtime = Runtime::new(Arc::new(TaskExecutor::new(4)));
        runtime.add_stage("first", &[], Touch(Shared::new(0u32), first_runs.clone()));
        runtime.add_stage("second", &[], Touch(Shared::new(0u64), second_runs.clone()));

        for _ in 0..50 {
            runtime.tick();
        }

        assert_eq!(first_runs.load(Ordering::SeqCst), 50);
        assert_eq!(second_runs.load(Ordering::SeqCst), 50);
    }
}
