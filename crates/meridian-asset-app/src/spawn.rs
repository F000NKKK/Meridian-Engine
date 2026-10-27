//! The port that runs work off the calling thread.

use crate::Priority;

/// Runs a job on a background thread. The adapter owns the threads
/// (`meridian-runtime-threads`); a job it never runs is dropped, which
/// [`AssetLoader`](crate::AssetLoader) turns into [`LoadError::Cancelled`](crate::LoadError).
pub trait Spawn: Send + Sync {
    /// Queues `job` to run once, as soon as a background thread is free.
    /// Of the queued jobs, a higher `priority` starts first.
    fn spawn(&self, priority: Priority, job: Box<dyn FnOnce() + Send>);
}
