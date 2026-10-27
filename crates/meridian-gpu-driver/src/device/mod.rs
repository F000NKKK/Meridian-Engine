//! The GPU device and its resource-creation methods.

mod bind_groups;
mod buffers;
mod device;
mod error;
mod shaders;
mod textures;

pub use device::Device;
pub use error::DeviceError;
