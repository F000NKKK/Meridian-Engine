//! The shared state every stage runs against.

use crate::AudioSubsystem;
use crate::PhysicsSubsystem;
use crate::StageContext;
use meridian_ecs_core::World;
use std::sync::{Arc, Mutex, MutexGuard};

/// Shared per-frame state, split into independently-lockable pieces —
/// see the module doc's "Locking" section for why this shape and not one
/// combined lock. `World` is available for application-level entity/
/// `Transform` use; not synced with `physics`'s bodies — no such mapping
/// is defined anywhere in the workspace yet, and inventing one here
/// would be new, undocumented design, not wiring together what already
/// exists.
#[derive(Debug, Clone)]
pub struct RuntimeState {
    world: Arc<Mutex<World>>,
    physics: Arc<Mutex<PhysicsSubsystem>>,
    audio: Arc<Mutex<AudioSubsystem>>,
}

impl RuntimeState {
    pub fn new(physics: PhysicsSubsystem, audio: AudioSubsystem) -> Self {
        Self {
            world: Arc::new(Mutex::new(World::new())),
            physics: Arc::new(Mutex::new(physics)),
            audio: Arc::new(Mutex::new(audio)),
        }
    }

    /// A [`StageContext`](crate::StageContext) holding clones of this state's locks — cheap
    /// (`Arc::clone`), one made fresh per stage per [`Runtime::tick`](crate::Runtime::tick)
    /// call.
    pub(crate) fn context(&self) -> StageContext {
        StageContext {
            world: self.world.clone(),
            physics: self.physics.clone(),
            audio: self.audio.clone(),
        }
    }

    /// Locks the world directly — for application code that isn't
    /// itself a [`Stage`](crate::Stage), between [`Runtime::tick`](crate::Runtime::tick) calls. No stage
    /// runs concurrently with application code between ticks, so
    /// there's no lock-order-inversion risk to guard against here the
    /// way [`StageContext`](crate::StageContext) does mid-tick — this is a plain, single
    /// lock.
    pub fn world(&self) -> MutexGuard<'_, World> {
        self.world
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Locks the physics state directly — see [`world`](Self::world)'s
    /// doc comment for the same "between ticks, not mid-tick" scoping.
    pub fn physics(&self) -> MutexGuard<'_, PhysicsSubsystem> {
        self.physics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Locks the audio state directly — see [`world`](Self::world)'s
    /// doc comment for the same "between ticks, not mid-tick" scoping.
    pub fn audio(&self) -> MutexGuard<'_, AudioSubsystem> {
        self.audio
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
