//! Spatial audio built on the GAC: mixer, DSP graph, listener and emitter frames.
//!
//! Layer: domain; bounded context: audio (`craftbook.toml`,
//! docs/context-map.md).
//!
//! Listener-local convention (deliberately chosen, not inherited from
//! anywhere else — `gac`'s `Vec3` has no built-in "forward"): facing
//! direction is local `+X`, up is local `+Y`, and `+X × +Y = +Z` (see
//! `gac::Vec3::cross`), so right is local `+Z`. A listener at
//! `Motor3::identity()` therefore faces world `+X` with world `+Z` to its
//! right — this is what every azimuth/panning calculation below is
//! defined against.
//!
//! Panning is horizontal-plane-only VBAP (Vector Base Amplitude Panning,
//! simplified to 2D): for a source azimuth, find the two speakers whose
//! azimuths bracket it and distribute constant-power gain between just
//! those two — zero everywhere else. One algorithm handles every layout
//! ([`SpeakerLayout::mono`]/[`stereo_headphones`](SpeakerLayout::stereo_headphones)/
//! [`stereo_speakers`](SpeakerLayout::stereo_speakers)/[`surround_5_0`](SpeakerLayout::surround_5_0)/
//! [`surround_5_1`](SpeakerLayout::surround_5_1)), but front-only layouts
//! (stereo/headphones — no rear speaker) and full-ring layouts (5.0/5.1 —
//! real rear speakers) need different edge behavior: see
//! [`SpeakerLayout::wraps_around`] and [`fold_to_front_hemisphere`].
//!
//! Design notes: docs/notes/meridian-audio-domain.md (`lib`).

#![forbid(unsafe_code)]

mod attenuation_model;
mod block_renderer;
mod channel;
pub mod effects;
mod emitter;
mod listener;
mod memory_source;
mod mixer;
#[cfg(test)]
mod render_and_output_tests;
mod sample_source;
mod spatial_gains;
mod speaker;
mod speaker_layout;
#[cfg(test)]
mod tests;
mod voice;
#[cfg(test)]
mod voice_tests;

pub use attenuation_model::AttenuationModel;
pub use block_renderer::BlockRenderer;
pub use channel::Channel;
pub use effects::{
    AcousticMedium, BinauralConfig, BinauralRenderer, Declicker, DspGraph, DspNode, Gain,
    LowPassFilter,
};
pub use emitter::Emitter;
pub use listener::Listener;
pub use memory_source::{MemorySource, Repeat};
pub use mixer::Mixer;
pub use sample_source::SampleSource;
pub use spatial_gains::{spatial_gains, spatial_gains_into};
pub use speaker::Speaker;
pub use speaker_layout::{SpeakerLayout, fold_to_front_hemisphere};
pub use voice::{PlaybackState, Voice};
