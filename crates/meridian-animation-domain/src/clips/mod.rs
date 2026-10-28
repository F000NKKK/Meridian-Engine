//! Keyframed clips and their playback.

mod channel;
mod clip;
mod error;
mod interpolate;
mod interpolation;
mod keys;
mod playback;
mod repeat;
mod track;
mod weights;
mod weights_channel;

pub use channel::Channel;
pub use clip::Clip;
pub use error::ClipError;
pub use interpolation::Interpolation;
pub use keys::Keys;
pub use playback::Playback;
pub use repeat::Repeat;
pub use track::Track;
pub use weights::WeightKeys;
pub use weights_channel::WeightsChannel;
