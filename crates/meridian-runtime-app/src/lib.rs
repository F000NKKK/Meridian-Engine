//! Runtime use cases: [`Runtime`] runs registered [`Stage`]s in dependency
//! order through the [`JobExecutor`] port, with the built-in
//! [`AudioStage`], [`PhysicsStepStage`] and [`ComputeStage`] and a typed [`EventSystem`].
//!
//! Layer: application; bounded context: runtime (`craftbook.toml`,
//! docs/context-map.md).
//!
//! This crate owns **no application state** (ADR 025): the composition root
//! creates the world, physics and audio state, wraps what several stages
//! share in [`Shared`], and gives each stage its handles. It names no
//! adapter: threads come from a [`JobExecutor`] (`meridian-runtime-threads`),
//! physics stepping from a `PhysicsStepper` (`meridian-physics-app`,
//! `meridian-physics-wgpu`), compute from a `ComputeContext`.
//!
//! Design notes: docs/notes/meridian-runtime-app.md (`lib`).

#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod audio_stage;
mod compute_stage;
mod event_system;
mod job;
mod job_executor;
mod physics_step_stage;
mod runtime;
mod shared;
mod stages;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
#[cfg(test)]
mod tests;

pub use audio_stage::AudioStage;
pub use compute_stage::ComputeStage;
pub use event_system::EventSystem;
pub use job::Job;
pub use job_executor::JobExecutor;
pub use physics_step_stage::PhysicsStepStage;
pub use runtime::Runtime;
pub use shared::Shared;
pub use stages::*;
