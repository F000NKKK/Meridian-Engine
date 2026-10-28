//! Playing sounds: a [`Voice`] pulls mono samples from a [`SampleSource`].

mod memory_source;
mod repeat;
mod resampled_source;
mod sample_source;
mod state;
#[cfg(test)]
mod tests;
mod voice;

pub use memory_source::MemorySource;
pub use repeat::Repeat;
pub use resampled_source::ResampledSource;
pub use sample_source::SampleSource;
pub use state::PlaybackState;
pub use voice::Voice;
