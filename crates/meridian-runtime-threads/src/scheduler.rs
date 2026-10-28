//! Runs a job graph across a pool of worker threads.

use std::panic::resume_unwind;
use std::sync::Arc;

use crate::JobGraph;
use crate::run_state::RunState;
use crate::worker_pool::WorkerPool;

/// Runs [`JobGraph`]s across worker threads, respecting declared
/// dependencies.
///
/// ## Implementation note
///
/// `worker_count - 1` threads are spawned once, in [`Scheduler::new`], and
/// parked between runs; the thread that calls [`run`](Self::run) is the
/// last worker. So a frame pays no thread-spawn cost (measured at ~210 µs
/// per run when threads were spawned per run). Every worker pulls from one
/// `Mutex`-guarded ready-queue rather than each having its own deque with
/// work-stealing; that is a throughput question to answer with
/// `cargo bench -p meridian-runtime-threads`, not before.
#[derive(Debug)]
pub struct Scheduler {
    worker_count: usize,
    pool: WorkerPool,
}

impl Scheduler {
    /// A scheduler with `worker_count` workers (at least 1), the calling
    /// thread of [`run`](Self::run) being one of them.
    #[must_use]
    pub fn new(worker_count: usize) -> Self {
        let worker_count = worker_count.max(1);
        Self {
            worker_count,
            pool: WorkerPool::new(worker_count - 1),
        }
    }

    /// The number of workers, the calling thread of `run` included.
    #[must_use]
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Runs every job in `graph` to completion. Blocks the calling thread
    /// until the whole graph has finished.
    ///
    /// # Panics
    ///
    /// Panics if `graph` contains a dependency cycle, or a [`JobId`](crate::JobId)
    /// that doesn't belong to it. If a job panics, the jobs not yet started
    /// are skipped and the first panic is re-raised here once the graph has
    /// drained.
    pub fn run(&self, graph: JobGraph) {
        let jobs = graph.len();
        if jobs == 0 {
            return;
        }
        let run = Arc::new(RunState::new(graph));
        self.pool.enlist(&run, jobs - 1);
        run.work();
        if let Some(panic) = run.take_panic() {
            resume_unwind(panic);
        }
    }
}
