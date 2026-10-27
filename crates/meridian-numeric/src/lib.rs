//! Scalar types, SIMD helpers, numeric traits and CPU feature dispatch underlying the GAC — kept separate so gac stays pure geometric algebra.
//!
//! Layer: shared kernel; bounded context: geometry (`craftbook.toml`,
//! docs/context-map.md).
//!
//! Two scalar flavors, in their own files: [`float`] (`Scalar = f32`, the
//! default, GPU-dispatchable path) and [`fixed`] ([`fixed::Fixed`], the
//! deterministic opt-in path — see that module's doc comment for why it
//! exists and when to reach for it). Both are re-exported at the crate
//! root for convenience.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub mod fixed;
pub mod float;
mod numeric_backend;
#[cfg(feature = "data")]
mod schema;

pub use fixed::Fixed;
pub use float::{EPSILON, Scalar, approx_eq};
pub use numeric_backend::NumericBackend;
