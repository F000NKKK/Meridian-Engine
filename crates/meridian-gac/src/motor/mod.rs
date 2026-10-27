//! The rigid-motion motor and the reference frame built on it, per flavor.

pub mod fixed;
pub mod float;
mod frame;

pub use frame::Frame;
