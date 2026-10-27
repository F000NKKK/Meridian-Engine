//! A stage that dispatches one compute kernel.

use meridian_compute_runtime::{ComputeKernel, ComputeScheduler, DispatchSize};

use crate::Stage;

/// Built-in convenience stage wrapping an arbitrary
/// `compute-runtime::ComputeKernel` — the concrete answer to "how does a
/// game hand the runtime its own GPU/CPU compute work," registered the
/// exact same way as any other [`Stage`](crate::Stage): dependency-ordered against
/// physics, audio, or other compute stages (a kernel that needs shared
/// state holds its own [`Shared`](crate::Shared) handle). Not
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
    /// `scheduler` is built by the composition root from its compute adapter.
    pub fn new(kernel: K, size: DispatchSize, scheduler: ComputeScheduler) -> Self {
        Self {
            scheduler,
            kernel,
            size,
        }
    }
}

impl<K: ComputeKernel + Send> Stage for ComputeStage<K> {
    fn run(&mut self) {
        self.scheduler.run(&self.kernel, self.size);
    }
}
