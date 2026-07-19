//! Scalar types, SIMD helpers, numeric traits and CPU feature dispatch underlying the GAC — kept separate so gac-core stays pure geometric algebra.

/// Workspace scalar type: the single source of truth for float precision.
pub type Scalar = f32;

/// Default tolerance for approximate floating-point comparisons workspace-wide.
pub const EPSILON: Scalar = 1e-5;

/// Approximate equality within [`EPSILON`].
pub fn approx_eq(a: Scalar, b: Scalar) -> bool {
    (a - b).abs() <= EPSILON
}

/// A numeric backend (scalar fallback vs. a SIMD-batched implementation).
pub trait NumericBackend {
    /// The batched representation this backend operates on.
    type Batch;

    fn feature_flags(&self) -> meridian_foundation::FeatureFlags;
}
