//! The error contract shared by every engine crate.

/// Marker trait every workspace error type implements, so crates can be
/// generic over "some engine error" without depending on a shared enum.
pub trait EngineError: core::fmt::Debug + core::fmt::Display {}
