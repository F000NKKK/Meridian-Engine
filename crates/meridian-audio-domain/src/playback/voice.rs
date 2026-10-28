//! One playing sound: a sample source bound to an emitter.

use super::{PlaybackState, SampleSource};
use crate::Emitter;

/// A sound source in the world: an [`Emitter`] frame, a gain, and the
/// [`SampleSource`] it plays. The application moves `emitter` each frame
/// (from physics, a script, a camera); the voice owns only playback.
pub struct Voice {
    /// Where the sound is, in world space.
    pub emitter: Emitter,
    /// Linear gain applied to every sample.
    pub gain: f32,
    state: PlaybackState,
    source: Box<dyn SampleSource>,
    block: Vec<f32>,
}

impl Voice {
    /// A playing voice at `emitter` with unit gain.
    #[must_use]
    pub fn new(source: impl SampleSource + 'static, emitter: Emitter) -> Self {
        Self {
            emitter,
            gain: 1.0,
            state: PlaybackState::Playing,
            source: Box::new(source),
            block: Vec::new(),
        }
    }

    /// The current playback state.
    #[must_use]
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Resumes a paused voice; a finished voice stays finished.
    pub fn play(&mut self) {
        if self.state == PlaybackState::Paused {
            self.state = PlaybackState::Playing;
        }
    }

    /// Silences a playing voice without losing its position.
    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    /// Ends the voice for good.
    pub fn stop(&mut self) {
        self.state = PlaybackState::Finished;
    }

    /// The next `frames` mono samples, gain applied. A paused or finished
    /// voice yields silence (a paused one does not advance its source); a
    /// source that ends mid-block is zero-padded and finishes the voice.
    /// The returned slice is valid until the next call; [`block`](Self::block)
    /// returns it again without advancing.
    pub fn render(&mut self, frames: usize) -> &[f32] {
        self.block.clear();
        self.block.resize(frames, 0.0);
        if self.state == PlaybackState::Playing {
            let read = self.source.read(&mut self.block);
            if read < frames {
                self.state = PlaybackState::Finished;
            }
            for sample in &mut self.block[..read] {
                *sample *= self.gain;
            }
        }
        &self.block
    }
}

impl Voice {
    /// The samples produced by the last [`render`](Self::render) call.
    #[must_use]
    pub fn block(&self) -> &[f32] {
        &self.block
    }
}

impl std::fmt::Debug for Voice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Voice")
            .field("emitter", &self.emitter)
            .field("gain", &self.gain)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}
