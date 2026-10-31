//! The unit of per-frame work.

/// One unit of per-frame work. Implementations are free to do anything —
/// step physics, mix audio, dispatch a compute kernel, present a rendered
/// frame, run gameplay logic — this crate has no opinion. A stage owns or
/// holds [`Shared`](crate::Shared) handles to exactly the state it uses.
/// `run` takes `&mut self` so a stage can hold private state across
/// frames (e.g. a `BinauralRenderer`'s delay lines, or a `ComputeStage`'s
/// `ComputeScheduler`).
pub trait Stage: Send {
    /// Runs this stage once.
    fn run(&mut self);

    /// Called by [`Runtime::resize_all`](crate::Runtime::resize_all) when
    /// the application's window changes size. Default no-op — most stages
    /// (physics, compute) have no notion of size; a rendering stage owning a
    /// swapchain-backed target overrides this to rebuild its size-dependent
    /// resources.
    fn resize(&mut self, _width: u32, _height: u32) {}
}
