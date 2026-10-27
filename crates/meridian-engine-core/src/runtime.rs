//! Registers stages and ticks them in dependency order.

use std::sync::{Arc, Mutex};

use crate::{Job, JobExecutor, Stage, StageEntry, StageId};

/// A set of registered [`Stage`]s run in dependency order every
/// [`Runtime::tick`] through a [`JobExecutor`]. It owns no application
/// state: each stage holds the [`Shared`](crate::Shared) handles it needs,
/// wired by the composition root.
pub struct Runtime {
    executor: Arc<dyn JobExecutor>,
    stages: Vec<StageEntry>,
}

impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("stages", &self.stages)
            .finish_non_exhaustive()
    }
}

impl Runtime {
    /// A runtime with no stages (see [`add_stage`](Self::add_stage)) that
    /// runs them on `executor`.
    #[must_use]
    pub fn new(executor: Arc<dyn JobExecutor>) -> Self {
        meridian_foundation::log_info!("engine-core Runtime initialized");
        Self {
            executor,
            stages: Vec::new(),
        }
    }

    /// Registers `stage`, runnable only once every stage in `deps` has
    /// completed this tick. `deps` must be [`StageId`]s this same `Runtime`
    /// already returned (stages are registered in dependency order).
    pub fn add_stage(
        &mut self,
        name: &'static str,
        deps: &[StageId],
        stage: impl Stage + 'static,
    ) -> StageId {
        let id = StageId(self.stages.len());
        self.stages.push(StageEntry {
            name,
            deps: deps.to_vec(),
            stage: Arc::new(Mutex::new(stage)),
        });
        id
    }

    /// Runs every registered stage once, in dependency order, independent
    /// branches in parallel if the executor allows. Builds a fresh job list
    /// every call (job actions are `FnOnce`, consumed once per frame).
    /// Blocks until every stage has finished.
    pub fn tick(&self) {
        let all_ids: Vec<StageId> = (0..self.stages.len()).map(StageId).collect();
        self.tick_only(&all_ids);
    }

    /// Like [`tick`](Self::tick), but runs only the stages named in `ids`
    /// (each still gated on any of *its own* dependencies that are also in
    /// `ids`; a dependency left out is treated as already satisfied). It
    /// exists because physics and rendering run at different
    /// multiplicities per display frame: a fixed-timestep accumulator
    /// calls the physics stage 0-8 times, while the render stage runs
    /// exactly once.
    ///
    /// # Panics
    ///
    /// Panics if a stage appears in `ids` before one of its dependencies
    /// that is also in `ids`.
    pub fn tick_only(&self, ids: &[StageId]) {
        let mut position: Vec<Option<usize>> = vec![None; self.stages.len()];
        let mut jobs = Vec::with_capacity(ids.len());
        for &id in ids {
            let entry = &self.stages[id.0];
            let deps = entry
                .deps
                .iter()
                .filter_map(|dep| {
                    let selected = ids.contains(dep);
                    selected.then(|| {
                        position[dep.0].expect(
                            "tick_only: a dependency must appear before its dependent in `ids`",
                        )
                    })
                })
                .collect();
            let stage = Arc::clone(&entry.stage);
            position[id.0] = Some(jobs.len());
            jobs.push(Job {
                name: entry.name,
                deps,
                action: Box::new(move || {
                    stage
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .run();
                }),
            });
        }
        self.executor.execute(jobs);
    }

    /// Calls [`Stage::resize`] on every registered stage, in registration
    /// order. Call this from an application's window-resize handler.
    pub fn resize_all(&self, width: u32, height: u32) {
        for entry in &self.stages {
            entry
                .stage
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .resize(width, height);
        }
    }
}
