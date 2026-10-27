//! The shared state of one `Scheduler::run`, and the loop every worker runs on it.

use std::any::Any;
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

use crate::JobGraph;
use crate::job_entry::Action;
use crate::topology::Topology;

type Panic = Box<dyn Any + Send>;

/// Everything that changes while a graph runs, behind one lock: in-degree
/// counts, the reverse-edge (dependents) list, each job's not-yet-taken
/// action, the queue of runnable job indices, how many jobs are still
/// outstanding, and the first panic a job raised.
struct Progress {
    indegree: Vec<usize>,
    dependents: Vec<Vec<usize>>,
    actions: Vec<Option<Action>>,
    ready: VecDeque<usize>,
    remaining: usize,
    panic: Option<Panic>,
}

/// One graph being run. Shared (`Arc`) between the thread that called
/// `Scheduler::run` and every pool worker enlisted for it.
pub(crate) struct RunState {
    progress: Mutex<Progress>,
    wake: Condvar,
}

impl RunState {
    /// # Panics
    ///
    /// On a malformed `graph`; see [`Topology::of`].
    pub(crate) fn new(mut graph: JobGraph) -> Self {
        let Topology {
            indegree,
            dependents,
        } = Topology::of(&graph);
        let ready = (0..indegree.len()).filter(|&i| indegree[i] == 0).collect();
        let actions = graph.jobs.iter_mut().map(|job| job.action.take()).collect();
        Self {
            progress: Mutex::new(Progress {
                remaining: indegree.len(),
                indegree,
                dependents,
                actions,
                ready,
                panic: None,
            }),
            wake: Condvar::new(),
        }
    }

    /// Takes jobs and runs them until the whole graph is finished. Safe to
    /// call from any number of threads at once, and after the graph is done
    /// (it then returns immediately).
    pub(crate) fn work(&self) {
        while let Some((index, action)) = self.next_job() {
            let outcome = action.map(|action| catch_unwind(AssertUnwindSafe(action)));
            self.finish(index, outcome.and_then(Result::err));
        }
    }

    /// The panic of the first job that panicked, if any. Call once the
    /// graph is done.
    pub(crate) fn take_panic(&self) -> Option<Panic> {
        self.lock().panic.take()
    }

    /// Blocks until a job is runnable (returning it) or the graph is done
    /// (`None`). Once a job has panicked, the actions of jobs not yet
    /// started are dropped unrun: the graph drains instead of continuing
    /// on top of a failed dependency.
    fn next_job(&self) -> Option<(usize, Option<Action>)> {
        let mut progress = self.lock();
        loop {
            if let Some(index) = progress.ready.pop_front() {
                let action = progress.actions[index].take();
                let action = action.filter(|_| progress.panic.is_none());
                return Some((index, action));
            }
            if progress.remaining == 0 {
                return None;
            }
            progress = self
                .wake
                .wait(progress)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Records `index` as done, releases the jobs that were waiting on it,
    /// and wakes the workers.
    fn finish(&self, index: usize, panic: Option<Panic>) {
        let mut progress = self.lock();
        if progress.panic.is_none() {
            progress.panic = panic;
        }
        let waiting = std::mem::take(&mut progress.dependents[index]);
        let mut released = 0usize;
        for dependent in waiting {
            progress.indegree[dependent] -= 1;
            if progress.indegree[dependent] == 0 {
                progress.ready.push_back(dependent);
                released += 1;
            }
        }
        progress.remaining -= 1;
        let done = progress.remaining == 0;
        drop(progress);
        if done {
            self.wake.notify_all();
        } else {
            // This worker takes one released job itself; wake one sleeper
            // per extra job instead of the whole pool.
            for _ in 1..released {
                self.wake.notify_one();
            }
        }
    }

    fn lock(&self) -> MutexGuard<'_, Progress> {
        self.progress.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
