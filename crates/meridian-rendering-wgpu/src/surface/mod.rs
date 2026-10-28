//! Where frames are drawn: a window's swapchain or an offscreen texture.

mod acquire_error;
mod frame;
mod offscreen;
mod swapchain;

pub use acquire_error::AcquireFrameError;
pub use frame::SurfaceFrame;
pub use swapchain::Surface;
pub(crate) use swapchain::SurfaceTarget;
