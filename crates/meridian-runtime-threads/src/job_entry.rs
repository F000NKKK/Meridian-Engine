//! One job in a graph: its name, its not-yet-run action and its dependencies.

use crate::job_id::JobId;

pub(crate) type Action = Box<dyn FnOnce() + Send>;

pub(crate) struct JobEntry {
    pub(crate) name: &'static str,
    pub(crate) action: Option<Action>,
    pub(crate) dependencies: Vec<JobId>,
}

impl std::fmt::Debug for JobEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobEntry")
            .field("name", &self.name)
            .field("has_action", &self.action.is_some())
            .field("dependencies", &self.dependencies)
            .finish()
    }
}
