//! Deformable-body physics — a mass-spring soft-body model, its own
//! submodule (not folded into the crate root) since it's a genuinely
//! separate domain from the rigid-body engine [`crate::generic`] owns
//! (different state shape — particles, not a collider transformed by one
//! frame — and a different collision response, per-particle rather than
//! whole-body impulse resolution). Same float/fixed split convention as
//! the rest of this crate: [`generic_softbody`] is the one, real,
//! `GaFlavor`-generic engine; [`float_softbody`]/[`fixed_softbody`] are
//! thin `FloatFlavor`/`FixedFlavor` aliases over it, not a second copy —
//! see CLAUDE.md's "Float/Fixed branching" rule. [`float_softbody`] is
//! re-exported at this module's own root (`physics_core::soft_body::SoftBody`
//! resolves to the `f32` path), mirroring how `crate::float` is
//! re-exported at the crate root.

pub mod fixed_softbody;
pub mod float_softbody;
pub mod generic_softbody;

pub use float_softbody::*;