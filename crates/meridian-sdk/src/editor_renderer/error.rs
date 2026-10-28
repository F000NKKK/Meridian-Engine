//! Why an editor renderer could not be built.

use meridian_rendering_wgpu::DeviceError;

/// Failure to open the renderer's GPU device.
#[derive(Debug, thiserror::Error)]
pub enum EditorRendererError {
    /// No usable GPU adapter or device.
    #[error("cannot open a GPU device: {0}")]
    Device(#[from] DeviceError),
    /// The helper runtime for the blocking constructor could not start.
    #[error("cannot start the async runtime: {0}")]
    Runtime(#[from] std::io::Error),
}
