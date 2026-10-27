//! Rendering use cases and the ports to the GPU: [`FrameRecorder`], [`TextureStore`] and [`MeshStore`],
//! plus the [`RenderSession`] that presents frames through them.
//!
//! Layer: application; bounded context: rendering (`craftbook.toml`,
//! docs/context-map.md).
//!
//! The ports are declared here, by the layer that needs them (ADR 019); the
//! `wgpu` adapter (`meridian-rendering-wgpu`) implements them. Fakes and
//! contract suites are in [`testing`] behind the `testing` feature.

#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod frame_outcome;
mod frame_recorder;
mod frame_request;
mod mesh_registry_store;
mod mesh_store;
mod render_session;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
mod texture_store;

pub use frame_outcome::FrameOutcome;
pub use frame_recorder::FrameRecorder;
pub use frame_request::FrameRequest;
pub use mesh_store::MeshStore;
pub use render_session::RenderSession;
pub use texture_store::TextureStore;
