//! Worker threads that live as long as their `Scheduler`.

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{Builder, JoinHandle};

use crate::run_state::RunState;

/// Threads parked between runs. Each has its own channel; a run hands a
/// worker the graph's [`RunState`] and the worker helps until it is done.
/// A thread that cannot be spawned is skipped: the caller of `run` works
/// too, so every graph still finishes, just with fewer helpers.
pub(crate) struct WorkerPool {
    senders: Vec<Sender<Arc<RunState>>>,
    handles: Vec<JoinHandle<()>>,
}

impl WorkerPool {
    pub(crate) fn new(threads: usize) -> Self {
        let mut pool = Self {
            senders: Vec::with_capacity(threads),
            handles: Vec::with_capacity(threads),
        };
        for index in 0..threads {
            let (sender, receiver) = mpsc::channel();
            let spawned = Builder::new()
                .name(format!("meridian-worker-{index}"))
                .spawn(move || serve(&receiver));
            if let Ok(handle) = spawned {
                pool.senders.push(sender);
                pool.handles.push(handle);
            }
        }
        pool
    }

    /// Asks up to `helpers` workers to join `run`.
    pub(crate) fn enlist(&self, run: &Arc<RunState>, helpers: usize) {
        for sender in self.senders.iter().take(helpers) {
            // Jobs run under `catch_unwind`, so a worker only ends when the
            // pool is dropped; a failed send means a bug, and the caller of
            // `run` would still finish the graph alone.
            let delivered = sender.send(Arc::clone(run)).is_ok();
            debug_assert!(delivered, "a pool worker ended before its pool");
        }
    }
}

impl std::fmt::Debug for WorkerPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerPool")
            .field("threads", &self.handles.len())
            .finish_non_exhaustive()
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.senders.clear();
        for handle in self.handles.drain(..) {
            let joined = handle.join();
            debug_assert!(joined.is_ok(), "a pool worker panicked outside a job");
        }
    }
}

fn serve(receiver: &Receiver<Arc<RunState>>) {
    while let Ok(run) = receiver.recv() {
        run.work();
    }
}
