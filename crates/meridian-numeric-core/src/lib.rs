//! Scalar types, SIMD helpers, numeric traits and CPU feature dispatch underlying the GAC — kept separate so gac-core stays pure geometric algebra.
//!
//! Two scalar flavors, in their own files: [`float`] (`Scalar = f32`, the
//! default, GPU-dispatchable path) and [`fixed`] ([`fixed::Fixed`], the
//! deterministic opt-in path — see that module's doc comment for why it
//! exists and when to reach for it). Both are re-exported at the crate
//! root for convenience.

pub mod fixed;
pub mod float;

pub use fixed::Fixed;
pub use float::{EPSILON, Scalar, approx_eq};

/// A numeric backend (scalar fallback vs. a SIMD-batched implementation).
pub trait NumericBackend {
    /// The batched representation this backend operates on.
    type Batch;

    fn feature_flags(&self) -> meridian_foundation::FeatureFlags;
}
