//! Keyframed animation of a model's joints.

mod channel;
mod clip;
mod interpolation;
mod property;

pub use channel::ModelChannel;
pub use clip::ModelClip;
pub use interpolation::ModelInterpolation;
pub use property::ModelProperty;
