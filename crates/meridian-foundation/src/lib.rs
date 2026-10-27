//! Zero-dependency foundational types shared workspace-wide: error conventions, basic type aliases, feature-detection primitives — plus the unified logging and crash-reporting diagnostics.
//!
//! Layer: shared kernel; bounded context: diagnostics (`craftbook.toml`,
//! docs/context-map.md).
//!
//! One module per concern ([`logging`], [`crash_reporting`]), re-exported
//! flat where it helps. This crate is the bottom of the dependency graph
//! (see docs/dependency-rules.md): any crate may take an edge to it, and
//! the diagnostics facilities are *why* they'd want to — every crate
//! logs through the same sink, and one panic hook captures the whole
//! process's post-mortem.

#![forbid(unsafe_code)]

pub mod crash_reporting;
mod engine_error;
mod feature_flags;
pub mod logging;

pub use crash_reporting::CrashReportConfig;
pub use engine_error::EngineError;
pub use feature_flags::FeatureFlags;
pub use logging::LogLevel;
