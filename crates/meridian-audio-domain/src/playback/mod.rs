//! Playing sounds: a [`Voice`] pulls mono samples from a [`SampleSource`].

mod memory_source;
mod playback_state;
mod repeat;
mod sample_source;
#[cfg(test)]
mod tests;
mod voice;

pub use memory_source::MemorySource;
pub use playback_state::PlaybackState;
pub use repeat::Repeat;
pub use sample_source::SampleSource;
pub use voice::Voice;
