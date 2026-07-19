//! Zero-dependency foundational types shared workspace-wide: error conventions, basic type aliases, feature-detection primitives.

/// Marker trait every workspace error type implements, so crates can be
/// generic over "some engine error" without depending on a shared enum.
pub trait EngineError: core::fmt::Debug + core::fmt::Display {}

/// CPU/platform feature flags detected once at startup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FeatureFlags {
    pub simd128: bool,
    pub simd256: bool,
}
