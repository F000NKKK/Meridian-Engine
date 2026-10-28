//! The waiting line of a [`BackgroundPool`](crate::BackgroundPool).

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

use meridian_asset_app::Priority;

/// A job a pool thread runs once.
pub(crate) type Job = Box<dyn FnOnce() + Send>;

/// One first-in-first-out line per [`Priority`], shared by the pool's threads.
#[derive(Default)]
pub(crate) struct JobQueue {
    lines: Mutex<Lines>,
    wake: Condvar,
}

#[derive(Default)]
struct Lines {
    low: VecDeque<Job>,
    normal: VecDeque<Job>,
    high: VecDeque<Job>,
    closed: bool,
}

impl Lines {
    fn line(&mut self, priority: Priority) -> &mut VecDeque<Job> {
        match priority {
            Priority::Low => &mut self.low,
            Priority::Normal => &mut self.normal,
            Priority::High => &mut self.high,
        }
    }

    fn next(&mut self) -> Option<Job> {
        self.high
            .pop_front()
            .or_else(|| self.normal.pop_front())
            .or_else(|| self.low.pop_front())
    }
}

impl JobQueue {
    fn lock(&self) -> MutexGuard<'_, Lines> {
        // The lock is never held across a job, so a poisoned guard still
        // holds consistent lines.
        self.lines.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Adds `job` behind the others of its priority.
    pub(crate) fn push(&self, priority: Priority, job: Job) {
        self.lock().line(priority).push_back(job);
        self.wake.notify_one();
    }

    /// Blocks for the next job: the oldest of the highest priority. `None`
    /// once the queue is closed and empty.
    pub(crate) fn pop(&self) -> Option<Job> {
        let mut lines = self.lock();
        loop {
            if let Some(job) = lines.next() {
                return Some(job);
            }
            if lines.closed {
                return None;
            }
            lines = self
                .wake
                .wait(lines)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Lets the threads finish what is queued, then end.
    pub(crate) fn close(&self) {
        self.lock().closed = true;
        self.wake.notify_all();
    }
}
