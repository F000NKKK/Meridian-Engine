//! A registered stage with its dependencies.

use crate::Stage;
use crate::StageId;
use std::sync::{Arc, Mutex};

pub(crate) struct StageEntry {
    pub(crate) name: &'static str,
    pub(crate) deps: Vec<StageId>,
    pub(crate) stage: Arc<Mutex<dyn Stage>>,
}

impl std::fmt::Debug for StageEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StageEntry")
            .field("name", &self.name)
            .field("deps", &self.deps)
            .finish_non_exhaustive()
    }
}
