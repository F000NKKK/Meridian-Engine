//! Scalar types, SIMD helpers, numeric traits and CPU feature dispatch underlying the GAC — kept separate so gac-core stays pure geometric algebra.

/// Workspace scalar type: the single source of truth for float precision.
pub type Scalar = f32;

/// A numeric backend (scalar fallback vs. a SIMD-batched implementation).
pub trait NumericBackend {
    /// The batched representation this backend operates on.
    type Batch;

    fn feature_flags(&self) -> meridian_foundation::FeatureFlags;
}
