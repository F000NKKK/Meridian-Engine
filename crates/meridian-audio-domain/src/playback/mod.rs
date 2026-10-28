//! Playing sounds: a [`Voice`] pulls mono samples from a [`SampleSource`].

mod memory_source;
mod repeat;
mod sample_source;
mod state;
#[cfg(test)]
mod tests;
mod voice;

pub use memory_source::MemorySource;
pub use repeat::Repeat;
pub use sample_source::SampleSource;
pub use state::PlaybackState;
pub use voice::Voice;
