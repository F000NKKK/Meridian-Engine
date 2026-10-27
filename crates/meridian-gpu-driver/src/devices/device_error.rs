//! Why a device could not be created.

/// Why [`Device::new`](crate::Device::new)/[`Device::new_windowed`](crate::Device::new_windowed) failed. Every variant is a
/// real, reachable failure — not a "shouldn't happen" case — since
/// adapter/device acquisition depends on what's actually installed on the
/// machine running this code (driver, ICD, `wgpu` backend availability).
#[derive(Debug)]
pub enum DeviceError {
    /// No adapter matched the request (no GPU, no compatible driver, or
    /// `wgpu` has no backend for this platform).
    NoAdapter,
    RequestDevice(wgpu::RequestDeviceError),
    /// [`Device::new_windowed`](crate::Device::new_windowed) only: the window/display handle couldn't
    /// be turned into a `wgpu::Surface` (e.g. an unsupported platform
    /// backend).
    CreateSurface(wgpu::CreateSurfaceError),
}

impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceError::NoAdapter => write!(f, "no compatible GPU adapter found"),
            DeviceError::RequestDevice(e) => write!(f, "failed to request GPU device: {e}"),
            DeviceError::CreateSurface(e) => write!(f, "failed to create GPU surface: {e}"),
        }
    }
}

impl std::error::Error for DeviceError {}

impl meridian_foundation::EngineError for DeviceError {}
