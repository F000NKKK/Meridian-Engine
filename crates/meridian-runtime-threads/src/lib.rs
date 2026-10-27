//! Job graph scheduler: worker threads, dependency tracking and parallel frame execution.
//!
//! Layer: infrastructure (adapter); bounded context: runtime
//! (`craftbook.toml`, docs/context-map.md).
//!
//! Systems declare a job's dependencies when adding it to a [`JobGraph`];
//! [`Scheduler::run`] derives execution order from that graph rather than
//! the caller hand-sequencing calls, and runs independent branches (see
//! docs/threading-model.md's shape-of-a-frame example) across worker
//! threads in parallel. The workers are spawned once with the
//! [`Scheduler`] and reused by every run; they share a single ready-queue
//! behind one mutex — see "Implementation note" on [`Scheduler`].
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod background_pool;
mod job_entry;
mod job_graph;
mod job_id;
mod job_queue;
mod run_state;
mod scheduler;
mod task_executor;
#[cfg(test)]
mod tests;
mod topology;
mod worker_pool;

pub use background_pool::BackgroundPool;
pub use job_graph::JobGraph;
pub use job_id::JobId;
pub use scheduler::Scheduler;
pub use task_executor::TaskExecutor;
