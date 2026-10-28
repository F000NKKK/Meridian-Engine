//! Fakes and contract suites for the rendering ports (Craftbook 8.4).

mod fake_frame_recorder;
mod fake_texture_store;
mod frame_recorder_contract;
mod mesh_store_contract;
mod texture_store_contract;

pub use fake_frame_recorder::FakeFrameRecorder;
pub use fake_texture_store::FakeTextureStore;
pub use frame_recorder_contract::run_frame_recorder_contract;
pub use mesh_store_contract::run_mesh_store_contract;
pub use texture_store_contract::run_texture_store_contract;
