//! Deformable-body physics: a mass-spring soft-body model, a submodule of its
//! own since it is a separate domain from the rigid-body engine in
//! [`crate::dynamics`] (different state shape, particles rather than a
//! collider moved by one frame, and a per-particle collision response).
//!
//! Same float/fixed convention as the rest of the crate: the engine
//! ([`spring`], [`mass_spring`], [`time_step`], [`ball`]) is written once,
//! generic over `F: GaFlavor`; [`float`] and [`fixed`] are thin aliases.
//! [`float`] is re-exported at this module's root, so
//! `physics_core::soft_body::SoftBody` resolves to the `f32` path, as
//! `crate::float` is re-exported at the crate root.

pub mod ball;
#[expect(
    clippy::module_name_repetitions,
    reason = "the `Fixed*` aliases sit in `fixed` because they are re-exported next to the float names"
)]
pub mod fixed;
pub mod float;
pub mod mass_spring;
pub mod spring;
pub mod time_step;

#[cfg(test)]
mod tests;

pub use float::*;
