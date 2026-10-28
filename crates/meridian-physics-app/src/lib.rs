//! Physics use cases: the [`PhysicsSubsystem`] world state, the
//! [`PhysicsStepper`] port and its [`SequentialStepper`], plus the
//! [`ContactWatcher`] that turns a step into [`ContactEvent`]s.
//!
//! Layer: application; bounded context: physics (`craftbook.toml`,
//! docs/context-map.md).
//!
//! The port is declared here, by the layer that needs it (ADR 019,
//! ADR 025); `meridian-physics-wgpu` implements it with batched compute
//! kernels. The runtime calls a stepper without knowing which one it got.
//! A contract suite is in [`testing`] behind the `testing` feature.

#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod contact_event;
mod contact_watcher;
mod physics_stepper;
mod physics_subsystem;
mod sequential_stepper;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use contact_event::ContactEvent;
pub use contact_watcher::ContactWatcher;
pub use physics_stepper::PhysicsStepper;
pub use physics_subsystem::{PhysicsSubsystem, RELAXATION_ITERATIONS};
pub use sequential_stepper::SequentialStepper;
