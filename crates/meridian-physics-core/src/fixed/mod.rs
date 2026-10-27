//! Thin `FixedFlavor` (`Fixed`-point) aliases over `crate::generic`'s
//! engine — an opt-in, bit-reproducible simulation path for lockstep
//! networking/replay (see `meridian_numeric_core::Fixed`'s doc comment
//! for what that buys and why plain `f32` can't). See `crate::generic`'s
//! module doc for why this is just aliases, not a second copy of the
//! engine: since the SAT-based cuboid narrow phase is exactly as generic
//! as the sphere path, it comes along for free — no "sphere only" scope
//! limit needed here, unlike the old hand-duplicated
//! `physics-core::deterministic` module this replaces.
//!
//! Nothing here changes how [`crate::float::RigidBody`]/
//! [`crate::float::Integrator`]/etc. behave; it's a parallel API, not a
//! mode switch on those types. Use
//! [`meridian_gac_core::fixed_ga::FixedMotor3::to_float_lossy`] (and the
//! matching methods on `FixedVec3`/`FixedBivector3`) to hand a
//! deterministically-computed pose off to rendering/ECS/audio, which
//! stay entirely in `f32` regardless of which physics path produced it.

pub use meridian_gac_core::fixed_ga::FixedFlavor;

pub type ColliderShape = crate::generic::ColliderShape<FixedFlavor>;
pub type RigidBody = crate::generic::RigidBody<FixedFlavor>;
pub type BroadPhase = crate::generic::BroadPhase<FixedFlavor>;
pub type Contact = crate::generic::Contact<FixedFlavor>;
pub type NarrowPhase = crate::generic::NarrowPhase<FixedFlavor>;
pub type ConstraintSolver = crate::generic::ConstraintSolver<FixedFlavor>;
pub type Integrator = crate::generic::Integrator<FixedFlavor>;

#[cfg(test)]
mod tests;
