//! Threads for blocking background work, apart from the frame workers.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};

use meridian_asset_app::{Priority, Spawn};

use crate::job_queue::{Job, JobQueue};

/// A fixed set of threads that run detached jobs, such as asset loads that
/// block on a disk or a network. It implements `asset-app`'s [`Spawn`] port.
///
/// Deliberately not the frame [`Scheduler`](crate::Scheduler)'s workers: a
/// job parked in a blocking read there would stall the frame's job graph
/// (ADR 027). Jobs start by priority, and in the order they were spawned
/// within one; a job that panics does not take its thread down.
pub struct BackgroundPool {
    queue: Arc<JobQueue>,
    handles: Vec<JoinHandle<()>>,
}

impl BackgroundPool {
    /// A pool of `threads` threads (at least 1). A thread that cannot be
    /// spawned is skipped, so the pool may be smaller; with none at all,
    /// jobs are dropped unrun and their `Pending` resolves as cancelled.
    #[must_use]
    pub fn new(threads: usize) -> Self {
        let queue = Arc::new(JobQueue::default());
        let handles = (0..threads.max(1))
            .filter_map(|index| {
                let queue = Arc::clone(&queue);
                Builder::new()
                    .name(format!("meridian-background-{index}"))
                    .spawn(move || serve(&queue))
                    .ok()
            })
            .collect();
        Self { queue, handles }
    }

    /// How many threads the pool has.
    #[must_use]
    pub fn thread_count(&self) -> usize {
        self.handles.len()
    }
}

impl Spawn for BackgroundPool {
    fn spawn(&self, priority: Priority, job: Job) {
        // With no thread at all the job is dropped unrun, which is how its
        // owner learns it will never run.
        if !self.handles.is_empty() {
            self.queue.push(priority, job);
        }
    }
}

impl std::fmt::Debug for BackgroundPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackgroundPool")
            .field("threads", &self.handles.len())
            .finish_non_exhaustive()
    }
}

impl Drop for BackgroundPool {
    /// Finishes the jobs already queued, then stops the threads.
    fn drop(&mut self) {
        self.queue.close();
        for handle in self.handles.drain(..) {
            let joined = handle.join();
            debug_assert!(joined.is_ok(), "a pool thread panicked outside a job");
        }
    }
}

fn serve(queue: &JobQueue) {
    // The job's owner learns of a panic through its own channel; this
    // thread only has to survive it.
    while let Some(job) = queue.pop() {
        let survived = catch_unwind(AssertUnwindSafe(job));
        drop(survived);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    use super::*;

    #[test]
    fn runs_every_job_on_its_threads() {
        let pool = BackgroundPool::new(4);
        let (done, results) = channel();
        for value in 0..100_u32 {
            let done = done.clone();
            pool.spawn(
                Priority::Normal,
                Box::new(move || {
                    done.send(value).expect("receiver alive");
                }),
            );
        }
        let mut seen: Vec<u32> = (0..100)
            .map(|_| {
                results
                    .recv_timeout(Duration::from_secs(10))
                    .expect("job ran")
            })
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..100).collect::<Vec<_>>());
    }

    #[test]
    fn a_panicking_job_does_not_stop_the_pool() {
        let pool = BackgroundPool::new(1);
        pool.spawn(Priority::Normal, Box::new(|| panic!("job failed")));
        let (done, result) = channel();
        pool.spawn(
            Priority::Normal,
            Box::new(move || done.send(7_u8).expect("receiver alive")),
        );
        assert_eq!(result.recv_timeout(Duration::from_secs(10)), Ok(7));
    }

    #[test]
    fn dropping_the_pool_finishes_the_queued_jobs() {
        let finished = Arc::new(AtomicUsize::new(0));
        let pool = BackgroundPool::new(2);
        for _ in 0..50 {
            let finished = Arc::clone(&finished);
            pool.spawn(
                Priority::Normal,
                Box::new(move || {
                    finished.fetch_add(1, Ordering::SeqCst);
                }),
            );
        }
        drop(pool);
        assert_eq!(finished.load(Ordering::SeqCst), 50);
    }

    #[test]
    fn higher_priority_jobs_start_first_and_equals_keep_their_order() {
        let pool = BackgroundPool::new(1);
        let (gate, open) = channel::<()>();
        pool.spawn(
            Priority::Normal,
            Box::new(move || {
                open.recv().expect("gate opened");
            }),
        );
        let (done, order) = channel();
        let queue = [
            (Priority::Low, "low"),
            (Priority::Normal, "normal-1"),
            (Priority::High, "high-1"),
            (Priority::Normal, "normal-2"),
            (Priority::High, "high-2"),
        ];
        for (priority, name) in queue {
            let done = done.clone();
            pool.spawn(priority, Box::new(move || done.send(name).expect("alive")));
        }
        gate.send(()).expect("worker waits");

        let ran: Vec<&str> = (0..5)
            .map(|_| {
                order
                    .recv_timeout(Duration::from_secs(10))
                    .expect("job ran")
            })
            .collect();
        assert_eq!(ran, ["high-1", "high-2", "normal-1", "normal-2", "low"]);
    }

    #[test]
    fn a_pool_always_has_at_least_one_thread() {
        assert!(BackgroundPool::new(0).thread_count() >= 1);
    }
}
