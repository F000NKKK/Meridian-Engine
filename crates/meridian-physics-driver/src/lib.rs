//! Execution backend for physics: memory backend, SIMD/GPU dispatch and synchronization. Owns no collision algorithms, BVH, or broad-phase structures — those belong to physics-core.
//!
//! No GPU backend yet — same call as `compute-driver`/`platform-core`'s
//! `Window`: needs an external crate or unsafe FFI, not taken on
//! speculatively. `PhysicsBackend` reports real CPU capability
//! (`std::thread::available_parallelism`) so `physics-core` can decide
//! batch-size cutoffs the same way `compute-runtime` does.

/// Selects/reports the execution backend for a physics step.
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
        let cpu_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
        Self { gpu: false, cpu_threads }
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
