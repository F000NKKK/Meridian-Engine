//! How the physics stage dispatches its work.

/// Selects [`PhysicsStepStage`](crate::PhysicsStepStage)'s dispatch shape — the flag this crate
/// uses instead of two separate `Stage` types: one type switching on this
/// enum is the same "one mechanism, not two" reasoning this crate applies
/// to `Runtime` itself, at the stage level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhysicsDispatchMode {
    /// Plain sequential CPU loop: [`PhysicsSubsystem::step`](crate::PhysicsSubsystem::step) as-is.
    #[default]
    Sequential,
    /// `meridian-physics-compute`'s kernels, dispatched through
    /// `ComputeContext::parallel_for`'s CPU backend (real multi-core
    /// parallelism, not single-threaded, but not literally
    /// GPU-executing — see [`PhysicsStepStage`](crate::PhysicsStepStage)'s doc for the disclosed
    /// gap between this and an eventual WGSL path).
    Batched,
}
