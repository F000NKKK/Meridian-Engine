//! Execution backend for physics: memory backend, SIMD/GPU dispatch and synchronization. Owns no collision algorithms, BVH, or broad-phase structures — those belong to physics-core.
//!
//! No GPU backend yet — same call as `compute-driver`/`platform-core`'s
//! `Window`: needs an external crate or unsafe FFI, not taken on
//! speculatively. `PhysicsBackend` reports real CPU capability via
//! `platform-core::detect_cpu_threads` so `physics-core` can decide
//! batch-size cutoffs the same way `compute-runtime` does.

use meridian_platform_core::BackendCapabilities;

/// Selects/reports the execution backend for a physics step. Implements
/// `platform-core`'s [`BackendCapabilities`] — the shape shared with
/// `compute-driver::ComputeCapabilities` and future `graphics-driver`/
/// `audio-driver` equivalents.
#[derive(Debug, Clone, Copy)]
pub struct PhysicsBackend {
    pub gpu: bool,
    pub cpu_threads: usize,
}

impl Default for PhysicsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsBackend {
    pub fn new() -> Self {
        Self { gpu: false, cpu_threads: meridian_platform_core::detect_cpu_threads() }
    }
}

impl BackendCapabilities for PhysicsBackend {
    fn gpu_available(&self) -> bool {
        self.gpu
    }

    fn cpu_threads(&self) -> usize {
        self.cpu_threads
    }
}

/// A synchronization point between a physics step and its consumers
/// (rendering, audio): a monotonically increasing generation, bumped once
/// per completed step. A consumer compares the generation it last read
/// against `current()` to know whether physics has advanced since —
/// cheaper than re-reading every `RigidBody` to check for changes.
#[derive(Debug, Default)]
pub struct PhysicsSync {
    generation: u64,
}

impl PhysicsSync {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current(&self) -> u64 {
        self.generation
    }

    /// Call once per completed physics step.
    pub fn advance(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }

    /// True if `last_seen` is behind the current generation.
    pub fn has_advanced_since(&self, last_seen: u64) -> bool {
        last_seen != self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_reports_no_gpu_and_at_least_one_thread() {
        let backend = PhysicsBackend::new();
        assert!(!backend.gpu);
        assert!(backend.cpu_threads >= 1);
    }

    #[test]
    fn sync_starts_at_zero_and_advances() {
        let mut sync = PhysicsSync::new();
        assert_eq!(sync.current(), 0);
        assert_eq!(sync.advance(), 1);
        assert_eq!(sync.current(), 1);
        assert_eq!(sync.advance(), 2);
    }

    #[test]
    fn has_advanced_since_detects_staleness() {
        let mut sync = PhysicsSync::new();
        let seen = sync.current();
        assert!(!sync.has_advanced_since(seen));
        sync.advance();
        assert!(sync.has_advanced_since(seen));
    }
}
