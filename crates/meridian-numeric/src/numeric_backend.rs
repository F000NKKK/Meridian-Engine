//! The numeric backend contract: float or fixed arithmetic.

/// A numeric backend (scalar fallback vs. a SIMD-batched implementation).
pub trait NumericBackend {
    /// The batched representation this backend operates on.
    type Batch;

    /// Returns the CPU features available to this backend.
    fn feature_flags(&self) -> meridian_diagnostics::FeatureFlags;
}
