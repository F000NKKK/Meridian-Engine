//! Execution backend for physics: memory backend, SIMD/GPU dispatch and synchronization. Owns no collision algorithms, BVH, or broad-phase structures — those belong to physics-core.

/// Selects the execution backend (CPU SIMD vs. GPU compute) for a physics step.
#[derive(Debug, Clone, Copy, Default)]
pub struct PhysicsBackend {
    pub gpu: bool,
}

/// A synchronization point between a physics step and its consumers
/// (rendering, audio).
#[derive(Debug, Default)]
pub struct PhysicsSync;
