//! Zero-dependency foundational types shared workspace-wide: error conventions, basic type aliases, feature-detection primitives — plus the unified logging and crash-reporting diagnostics.
//!
//! One module per concern ([`logging`], [`crash_reporting`]), re-exported
//! flat where it helps. This crate is the bottom of the dependency graph
//! (see docs/dependency-rules.md): any crate may take an edge to it, and
//! the diagnostics facilities are *why* they'd want to — every crate
//! logs through the same sink, and one panic hook captures the whole
//! process's post-mortem.

pub mod crash_reporting;
pub mod logging;

pub use crash_reporting::CrashReportConfig;
pub use logging::LogLevel;

/// Marker trait every workspace error type implements, so crates can be
/// generic over "some engine error" without depending on a shared enum.
pub trait EngineError: core::fmt::Debug + core::fmt::Display {}

/// CPU/platform feature flags detected once at startup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FeatureFlags {
    pub simd128: bool,
    pub simd256: bool,
}
