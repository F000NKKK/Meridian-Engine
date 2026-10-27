//! A stage that dispatches one compute kernel.

use crate::Stage;
use crate::StageContext;
use meridian_compute_runtime::{ComputeKernel, ComputeScheduler, DispatchSize};

/// Built-in convenience stage wrapping an arbitrary
/// `compute-runtime::ComputeKernel` — the concrete answer to "how does a
/// game hand the runtime its own GPU/CPU compute work," registered the
/// exact same way as any other [`Stage`](crate::Stage): dependency-ordered against
/// physics, audio, or other compute stages, with `StageContext` giving
/// it nothing it doesn't ask for (a kernel that needs `physics`/`audio`
/// state locks them itself inside `run`, same as any other `Stage`). Not
/// specific to any domain — `gac-compute`/`physics-compute`'s own
/// kernels work here exactly as well as a game-specific one; this crate
/// still doesn't know what any given kernel actually computes.
#[derive(Debug)]
pub struct ComputeStage<K> {
    scheduler: ComputeScheduler,
    kernel: K,
    size: DispatchSize,
}

impl<K: ComputeKernel + Send + 'static> ComputeStage<K> {
    /// `size` is the dispatch shape passed to `ComputeKernel::dispatch`
    /// every tick — most kernels use `DispatchSize::linear(item_count)`
    /// (see that constructor's own doc comment).
    pub fn new(kernel: K, size: DispatchSize) -> Self {
        Self {
            scheduler: ComputeScheduler::new(),
            kernel,
            size,
        }
    }
}

impl<K: ComputeKernel + Send> Stage for ComputeStage<K> {
    fn run(&mut self, _ctx: &StageContext) {
        self.scheduler.run(&self.kernel, self.size);
    }
}
