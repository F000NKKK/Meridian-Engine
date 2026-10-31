//! The rotation rotor, per flavor.

pub mod fixed;
pub mod float;

mod checked;
mod rotation_error;
pub use rotation_error::RotationError;
