//! A free-fly camera and the device-independent input it reads.

mod camera;
mod controls;
#[cfg(test)]
mod tests;

pub use camera::FlyCamera;
pub use controls::FlyControls;
