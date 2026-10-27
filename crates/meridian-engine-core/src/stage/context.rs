//! The handle a stage receives to reach the shared state.

use crate::AudioSubsystem;
use crate::PhysicsSubsystem;
use meridian_ecs_core::World;
use std::sync::{Arc, Mutex, MutexGuard};

/// What a running [`Stage`](crate::Stage) sees — locked access to [`RuntimeState`](crate::RuntimeState)'s
/// pieces, shaped so a lock-order inversion can't be expressed (see the
/// module doc's "Locking" section).
#[derive(Debug)]
pub struct StageContext {
    pub(crate) world: Arc<Mutex<World>>,
    pub(crate) physics: Arc<Mutex<PhysicsSubsystem>>,
    pub(crate) audio: Arc<Mutex<AudioSubsystem>>,
}

impl StageContext {
    /// Locks only the world state.
    pub fn world(&self) -> MutexGuard<'_, World> {
        self.world
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Locks only the physics state. Blocks if another stage currently
    /// holds it (either a genuinely concurrent independent stage, or —
    /// if this stage depends on that one — this call simply never
    /// contends, since the dependency already guaranteed the earlier
    /// stage finished and dropped its guard first).
    pub fn physics(&self) -> MutexGuard<'_, PhysicsSubsystem> {
        self.physics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Locks only the audio state.
    pub fn audio(&self) -> MutexGuard<'_, AudioSubsystem> {
        self.audio
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Locks both physics and audio, always physics-then-audio
    /// internally — one of the only two ways this module lets a stage
    /// hold more than one lock at once, precisely so no caller can
    /// independently choose (and accidentally invert) the acquisition
    /// order.
    pub fn physics_and_audio(
        &self,
    ) -> (
        MutexGuard<'_, PhysicsSubsystem>,
        MutexGuard<'_, AudioSubsystem>,
    ) {
        let physics = self
            .physics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let audio = self
            .audio
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (physics, audio)
    }

    /// Locks all three, always world-then-physics-then-audio internally
    /// — the other of the only two multi-lock accessors this module
    /// exposes, same reasoning as
    /// [`physics_and_audio`](Self::physics_and_audio).
    pub fn world_physics_and_audio(
        &self,
    ) -> (
        MutexGuard<'_, World>,
        MutexGuard<'_, PhysicsSubsystem>,
        MutexGuard<'_, AudioSubsystem>,
    ) {
        let world = self
            .world
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let physics = self
            .physics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let audio = self
            .audio
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (world, physics, audio)
    }
}
