//! A free-fly camera and the device-independent input it reads.

mod fly_camera;
mod fly_controls;
#[cfg(test)]
mod tests;

pub use fly_camera::FlyCamera;
pub use fly_controls::FlyControls;
