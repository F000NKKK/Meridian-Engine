//! `Runtime`: the single entry point tying every other crate into the
//! main loop. Owns real instances of the driver-independent subsystems
//! that exist today — an `ecs-core` [`World`](meridian_ecs_core::World), [`PhysicsSubsystem`]
//! (`physics-core`'s body list, broad/narrow-phase, solver/integrator)
//! and [`AudioSubsystem`] (`audio-core`'s listener, emitters, mixer) —
//! this is still the one place in the workspace allowed to know about
//! every `*-core` at once (see docs/dependency-rules.md rule 7).
//!
//! Layer: application; bounded context: runtime (`craftbook.toml`,
//! docs/context-map.md).
//!
//! Design notes: docs/notes/meridian-engine-core.md (`lib`).

#![forbid(unsafe_code)]

mod audio_subsystem;
mod compute_stage;
mod event_system;
mod frame_scheduler;
mod physics_dispatch_mode;
mod physics_step_stage;
mod physics_subsystem;
mod runtime;
mod runtime_state;
mod stage;
#[cfg(test)]
mod tests;

pub use audio_subsystem::AudioSubsystem;
pub use compute_stage::ComputeStage;
pub use event_system::EventSystem;
pub use frame_scheduler::FrameScheduler;
pub use physics_dispatch_mode::PhysicsDispatchMode;
pub use physics_step_stage::PhysicsStepStage;
pub use physics_subsystem::PhysicsSubsystem;
pub(crate) use physics_subsystem::RELAXATION_ITERATIONS;
pub use runtime::Runtime;
pub use runtime_state::RuntimeState;
pub use stage::*;
