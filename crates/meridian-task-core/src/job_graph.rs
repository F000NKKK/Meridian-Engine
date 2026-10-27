//! The job graph: named jobs and the dependencies between them.

use crate::job_entry::JobEntry;
use crate::job_id::JobId;

/// A dependency-ordered set of jobs for one frame. Build with
/// [`add_job`](Self::add_job), then hand to [`Scheduler::run`](crate::Scheduler::run).
#[derive(Debug, Default)]
pub struct JobGraph {
    pub(crate) jobs: Vec<JobEntry>,
}

impl JobGraph {
    /// An empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many jobs the graph holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    /// Whether the graph holds no jobs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// The name a job was given when added — for logging/debugging which
    /// job is running, not used by the scheduler itself.
    #[must_use]
    pub fn job_name(&self, id: JobId) -> &'static str {
        self.jobs[id.0].name
    }

    /// Adds a job that becomes runnable only once every job in
    /// `dependencies` has completed. `dependencies` must be [`JobId`]s
    /// returned earlier from this same graph.
    pub fn add_job(
        &mut self,
        name: &'static str,
        dependencies: &[JobId],
        action: impl FnOnce() + Send + 'static,
    ) -> JobId {
        let id = JobId(self.jobs.len());
        self.jobs.push(JobEntry {
            name,
            action: Some(Box::new(action)),
            dependencies: dependencies.to_vec(),
        });
        id
    }
}
