//! Registers stages and ticks them in dependency order.

use crate::AudioSubsystem;
use crate::FrameScheduler;
use crate::PhysicsSubsystem;
use crate::RuntimeState;
use crate::Stage;
use crate::StageEntry;
use crate::StageId;
use meridian_platform_core::CpuCapabilities;
use meridian_task_core::JobGraph;
use std::sync::{Arc, Mutex};

/// Owns [`RuntimeState`](crate::RuntimeState) and a set of registered [`Stage`](crate::Stage)s, and runs
/// them in dependency order every [`Runtime::tick`] — see the module doc
/// for the `JobGraph`-based mechanism and the locking guarantee. The
/// single entry point for a frame's work in this workspace — see the
/// module doc's "one mechanism, not two" section.
#[derive(Debug)]
pub struct Runtime {
    state: RuntimeState,
    scheduler: FrameScheduler,
    stages: Vec<StageEntry>,
}

impl Runtime {
    /// Builds a `Runtime` with no stages registered yet (see
    /// [`add_stage`](Self::add_stage)) and a [`FrameScheduler`](crate::FrameScheduler) sized to
    /// the real detected CPU thread count. Use
    /// [`with_worker_count`](Self::with_worker_count) to size it
    /// explicitly instead (e.g. a fixed worker count for reproducible
    /// benchmarks).
    pub fn new(physics: PhysicsSubsystem, audio: AudioSubsystem) -> Self {
        Self::with_worker_count(physics, audio, CpuCapabilities::detect().threads)
    }

    pub fn with_worker_count(
        physics: PhysicsSubsystem,
        audio: AudioSubsystem,
        worker_count: usize,
    ) -> Self {
        meridian_foundation::log_info!("engine-core Runtime initialized");
        Self {
            state: RuntimeState::new(physics, audio),
            scheduler: FrameScheduler::new(worker_count),
            stages: Vec::new(),
        }
    }

    /// The shared state this runtime's stages run against — clone it
    /// (cheap: `Arc` clones) to read/write it directly between ticks,
    /// e.g. to seed initial bodies or read back positions after
    /// [`tick`](Self::tick) returns.
    pub fn state(&self) -> &RuntimeState {
        &self.state
    }

    /// Registers `stage`, runnable only once every stage in `deps` has
    /// completed this tick — the same contract
    /// `meridian_task_core::JobGraph::add_job` documents, one layer
    /// down. `deps` must be [`StageId`](crate::StageId)s this same `Runtime` already
    /// returned (stages are registered in dependency order, same as
    /// `JobGraph` itself).
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

    /// Runs every registered stage once, in dependency order,
    /// independent branches in parallel — rebuilds a fresh `JobGraph`
    /// every call (matching `JobGraph`'s own "a dependency-ordered set
    /// of jobs for one frame" contract — its job actions are `FnOnce`,
    /// consumed once by `Scheduler::run`, not meant to be reused across
    /// frames). Blocks until every stage has finished.
    pub fn tick(&self) {
        let all_ids: Vec<StageId> = (0..self.stages.len()).map(StageId).collect();
        self.tick_only(&all_ids);
    }

    /// Like [`tick`](Self::tick), but runs only the stages named in
    /// `ids` (each still gated on any of *its own* dependencies that are
    /// also in `ids` — a dependency left out of `ids` is treated as
    /// already satisfied, not an error, since the whole point of this
    /// method is running a subset on its own schedule). The concrete
    /// reason this exists rather than every application always calling
    /// plain [`tick`](Self::tick): physics and rendering genuinely run
    /// at different multiplicities per real display frame — a
    /// fixed-timestep physics accumulator calls its stage 0-8 times to
    /// catch up on one real frame's elapsed time, while a rendering
    /// stage must run exactly once per display frame (a
    /// `Runtime::tick`-per-catch-up-step render would present several
    /// swapchain frames per real frame, wrong and wasteful). Both still
    /// go through this same `Runtime` and the same dependency-graph
    /// mechanism — `tick_only(&[physics_id])` in the accumulator loop,
    /// `tick_only(&[render_id])` once after it, rather than either stage
    /// being invoked outside `Runtime` entirely.
    pub fn tick_only(&self, ids: &[StageId]) {
        let selected: std::collections::HashSet<usize> = ids.iter().map(|id| id.0).collect();
        let mut graph = JobGraph::new();
        let mut job_ids: Vec<Option<meridian_task_core::JobId>> = vec![None; self.stages.len()];

        for &id in ids {
            let entry = &self.stages[id.0];
            let dep_job_ids: Vec<_> = entry
                .deps
                .iter()
                .filter(|dep| selected.contains(&dep.0))
                .map(|dep| {
                    job_ids[dep.0]
                        .expect("tick_only: a dependency must appear before its dependent in `ids`")
                })
                .collect();
            let stage = entry.stage.clone();
            let ctx = self.state.context();
            let job_id = graph.add_job(entry.name, &dep_job_ids, move || {
                stage
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .run(&ctx);
            });
            job_ids[id.0] = Some(job_id);
        }

        self.scheduler.run(graph);
    }

    /// Calls [`Stage::resize`](crate::Stage::resize) on every registered stage, in
    /// registration order (no dependency graph needed here — resize
    /// isn't a per-frame data-flow operation, and stages don't depend on
    /// each other's resize side effects the way [`tick`](Self::tick)'s
    /// stages depend on each other's *data*). Call this from an
    /// application's window-resize handler.
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
