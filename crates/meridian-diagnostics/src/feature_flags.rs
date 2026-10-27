//! Compile-time feature switches.

/// CPU/platform feature flags detected once at startup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FeatureFlags {
    pub simd128: bool,
    pub simd256: bool,
}
