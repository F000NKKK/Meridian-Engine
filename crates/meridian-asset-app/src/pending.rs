//! A result that a background job will fill in.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

/// What a background job has produced so far.
struct Slot<T> {
    value: Mutex<Option<T>>,
    ready: Condvar,
    cancelled: AtomicBool,
}

impl<T> Slot<T> {
    fn lock(&self) -> MutexGuard<'_, Option<T>> {
        // A job never panics while holding the lock, but a poisoned lock
        // still holds a valid `Option`, so keep going rather than fail.
        self.value
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// The caller's side of a background job: poll it each frame with
/// [`take`](Pending::take) or block for it with [`wait`](Pending::wait).
pub struct Pending<T> {
    slot: Arc<Slot<T>>,
}

/// The job's side. Dropping it without calling [`complete`](Completer::complete)
/// (the pool went away, or the job panicked) fills the slot with the fallback
/// value, so a [`Pending`] always resolves.
pub(crate) struct Completer<T> {
    slot: Arc<Slot<T>>,
    fallback: fn() -> T,
    done: bool,
}

/// A linked [`Pending`] and [`Completer`]. `fallback` is the value of a job
/// that never completed.
pub(crate) fn channel<T>(fallback: fn() -> T) -> (Pending<T>, Completer<T>) {
    let slot = Arc::new(Slot {
        value: Mutex::new(None),
        ready: Condvar::new(),
        cancelled: AtomicBool::new(false),
    });
    let pending = Pending {
        slot: Arc::clone(&slot),
    };
    (
        pending,
        Completer {
            slot,
            fallback,
            done: false,
        },
    )
}

impl<T> Completer<T> {
    /// Whether the caller asked for the job to be skipped.
    pub(crate) fn is_cancelled(&self) -> bool {
        self.slot.cancelled.load(Ordering::Acquire)
    }

    pub(crate) fn complete(mut self, value: T) {
        self.fill(value);
    }

    fn fill(&mut self, value: T) {
        *self.slot.lock() = Some(value);
        self.done = true;
        self.slot.ready.notify_all();
    }
}

impl<T> Drop for Completer<T> {
    fn drop(&mut self) {
        if !self.done {
            let value = (self.fallback)();
            self.fill(value);
        }
    }
}

impl<T> Pending<T> {
    /// Whether the result is there (a following [`take`](Self::take) returns it).
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.slot.lock().is_some()
    }

    /// The result if the job has finished, without blocking. Returns it once:
    /// afterwards, and while the job is still running, it is `None`.
    #[must_use]
    pub fn take(&self) -> Option<T> {
        self.slot.lock().take()
    }

    /// Asks the job not to run. A job still queued is skipped and resolves
    /// as cancelled; one that already started finishes and keeps its result.
    /// Either way the [`Pending`] still resolves, so `wait` does not hang.
    pub fn cancel(&self) {
        self.slot.cancelled.store(true, Ordering::Release);
    }

    /// Blocks until the job has finished and returns its result.
    ///
    /// # Panics
    ///
    /// If [`take`](Self::take) already returned the result.
    #[must_use]
    pub fn wait(self) -> T {
        let mut guard = self.slot.lock();
        loop {
            if let Some(value) = guard.take() {
                return value;
            }
            guard = self
                .slot
                .ready
                .wait(guard)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
}

impl<T> std::fmt::Debug for Pending<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pending")
            .field("ready", &self.is_ready())
            .finish()
    }
}
