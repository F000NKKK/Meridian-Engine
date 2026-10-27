//! Audio use cases and the ports to the outside world: the [`AudioSink`] mixed samples leave through.
//!
//! Layer: application; bounded context: audio (`craftbook.toml`,
//! docs/context-map.md).
//!
//! The port is declared here, by the layer that needs it (ADR 019); the platform
//! adapter (`meridian-audio-driver`) implements it. A fake and a contract suite
//! are in [`testing`] behind the `testing` feature.

#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod audio_sink;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use audio_sink::AudioSink;
