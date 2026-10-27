//! Zero-dependency foundational types shared workspace-wide: error conventions, feature-detection primitives and the `LogSink`-based logging kernel.
//!
//! Layer: shared kernel; bounded context: diagnostics (`craftbook.toml`,
//! docs/context-map.md).
//!
//! The log kernel ([`logging`]) formats lines and keeps a ring; file output
//! and crash reports are `meridian-diagnostics-fs`. This crate is the bottom of the dependency graph
//! (see docs/dependency-rules.md): any crate may take an edge to it, and
//! every crate logs through the same sink.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod engine_error;
mod feature_flags;
pub mod logging;

pub use engine_error::EngineError;
pub use feature_flags::FeatureFlags;
pub use logging::LogLevel;
