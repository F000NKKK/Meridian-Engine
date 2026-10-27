//! The rendering context's ports: what the domain asks of a GPU adapter.

mod frame_outcome;
mod frame_recorder;
mod frame_request;
mod mesh_store;
mod texture_store;

pub use frame_outcome::FrameOutcome;
pub use frame_recorder::FrameRecorder;
pub use frame_request::FrameRequest;
pub use mesh_store::MeshStore;
pub use texture_store::TextureStore;
