//! Substrate Data schemas for the GA value types (`data` feature): each is
//! a `NamedType` (`vec3`, `bivector3`, `rotor`, `motor`, `fixed_*`) stored
//! in `gac-core`'s own component order, bit-exactly in every form. Formats
//! and rationale: docs/data-formats.md ("Math values").
//!
//! The logic is written once, generic over the component scalar
//! ([`StoredScalar`]): [`blades`] maps multivectors to stored components,
//! [`MotorForm`] is a motor's stored form. [`float`] and [`fixed`] only
//! bind each concrete type to them (the float/fixed rule of CLAUDE.md).

mod blades;
mod fixed;
mod float;
mod motor_blades;
mod motor_form;
mod stored_scalar;
#[cfg(test)]
mod tests;

use motor_blades::MotorBlades;
pub use motor_form::MotorForm;
use stored_scalar::StoredScalar;
