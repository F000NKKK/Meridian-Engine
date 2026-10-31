//! Compile-time feature switches.

/// CPU/platform feature flags detected once at startup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FeatureFlags {
    /// Whether 128-bit SIMD instructions are available.
    pub simd128: bool,
    /// Whether 256-bit SIMD instructions are available.
    pub simd256: bool,
}
