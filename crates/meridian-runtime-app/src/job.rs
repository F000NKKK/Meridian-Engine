//! One unit of work handed to a job executor.

/// A named action plus the jobs that must finish before it starts.
///
/// `deps` are indices into the same `Vec<Job>` passed to
/// [`JobExecutor::execute`](crate::JobExecutor::execute), and each must be
/// smaller than the job's own index (jobs are listed in dependency order).
pub struct Job {
    pub name: &'static str,
    pub deps: Vec<usize>,
    pub action: Box<dyn FnOnce() + Send>,
}

impl std::fmt::Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("name", &self.name)
            .field("deps", &self.deps)
            .finish_non_exhaustive()
    }
}
