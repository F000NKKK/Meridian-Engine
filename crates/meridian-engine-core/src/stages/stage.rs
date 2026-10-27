//! The unit of per-frame work.

use crate::StageContext;

/// One unit of per-frame work. Implementations are free to do anything —
/// step physics, mix audio, dispatch a compute kernel, present a
/// rendered frame, run gameplay logic — this crate has no opinion; see
/// the module doc for why that's the point. `run` takes `&mut self` so a
/// stage can hold its own private state across frames (e.g. a
/// `BinauralRenderer`'s internal delay lines, or a `ComputeStage`'s
/// `ComputeScheduler`, which must persist call-to-call).
pub trait Stage: Send {
    fn run(&mut self, ctx: &StageContext);

    /// Called by [`Runtime::resize_all`](crate::Runtime::resize_all) when the application's window
    /// changes size. Default no-op — most stages (physics, compute) have
    /// no notion of size; a rendering stage (owning a swapchain-backed
    /// target) overrides this to rebuild its size-dependent resources.
    /// Generalizing resize to *every* stage, rather than a
    /// rendering-specific side channel, is what lets a render-presenting
    /// `Stage` (see `meridian-sdk`'s own module doc) live in the same
    /// registry as every other stage instead of needing special-cased
    /// access from application code.
    fn resize(&mut self, _width: u32, _height: u32) {}
}
