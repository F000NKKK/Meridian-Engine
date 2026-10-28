//! Keyframed animation of a model's joints.

mod channel;
mod clip;
mod interpolation;
mod property;
mod weights_channel;

pub use channel::ModelChannel;
pub use clip::ModelClip;
pub use interpolation::ModelInterpolation;
pub use property::ModelProperty;
pub use weights_channel::ModelWeightsChannel;
