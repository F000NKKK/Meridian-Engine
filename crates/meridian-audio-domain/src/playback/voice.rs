//! One playing sound: a sample source bound to an emitter.

use super::{PlaybackState, SampleSource};
use crate::Emitter;

/// A sound source in the world: an [`Emitter`] frame, a gain, and the
/// [`SampleSource`] it plays. The application moves `emitter` each frame
/// (from physics, a script, a camera); the voice owns only playback.
///
/// Starting, pausing, resuming and stopping can fade over a configured
/// number of frames ([`with_fade`](Self::with_fade)) instead of cutting the
/// waveform, which would click. [`state`](Self::state) changes at once; the
/// audible tail of a fade-out continues until [`is_done`](Self::is_done).
pub struct Voice {
    /// Where the sound is, in world space.
    pub emitter: Emitter,
    /// Linear gain applied to every sample.
    pub gain: f32,
    state: PlaybackState,
    source: Box<dyn SampleSource>,
    block: Vec<f32>,
    fade: Fade,
    source_ended: bool,
}

/// The audible level ramping toward a target.
#[derive(Debug, Clone, Copy)]
struct Fade {
    level: f32,
    target: f32,
    /// Level change per frame; `1.0` or more is an instant cut.
    step: f32,
}

impl Fade {
    fn advance(&mut self) -> f32 {
        let delta = self.target - self.level;
        self.level = if delta.abs() <= self.step {
            self.target
        } else {
            self.level + self.step.copysign(delta)
        };
        self.level
    }

    /// Aims at `target`; an instant fade jumps there.
    fn retarget(&mut self, target: f32) {
        self.target = target;
        if self.step >= 1.0 {
            self.level = target;
        }
    }

    fn is_silent(self) -> bool {
        self.level == 0.0 && self.target == 0.0
    }
}

impl Voice {
    /// A playing voice at `emitter` with unit gain and no fades.
    #[must_use]
    pub fn new(source: impl SampleSource + 'static, emitter: Emitter) -> Self {
        Self {
            emitter,
            gain: 1.0,
            state: PlaybackState::Playing,
            source: Box::new(source),
            block: Vec::new(),
            fade: Fade {
                level: 1.0,
                target: 1.0,
                step: 1.0,
            },
            source_ended: false,
        }
    }

    /// Fades in from silence, and fades out on pause and stop, over `frames`
    /// frames each way. `0` means no fade.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "fade lengths are a few thousand frames; f32 is exact there"
    )]
    pub fn with_fade(mut self, frames: usize) -> Self {
        if frames > 0 {
            self.fade = Fade {
                level: 0.0,
                target: 1.0,
                step: 1.0 / frames as f32,
            };
        }
        self
    }

    /// The current playback state.
    #[must_use]
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Whether the voice has ended and nothing of it is still audible, so a
    /// scene may drop it.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.state == PlaybackState::Finished && self.fade.is_silent()
    }

    /// Resumes a paused voice (fading in); a finished voice stays finished.
    pub fn play(&mut self) {
        if self.state == PlaybackState::Paused {
            self.state = PlaybackState::Playing;
            self.fade.retarget(1.0);
        }
    }

    /// Silences a playing voice (fading out) without losing its position.
    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
            self.fade.retarget(0.0);
        }
    }

    /// Ends the voice for good, fading out first if a fade is configured.
    pub fn stop(&mut self) {
        self.state = PlaybackState::Finished;
        self.fade.retarget(0.0);
    }

    /// The next `frames` mono samples, gain and fade applied. A paused voice
    /// is silent once its fade-out ends and does not advance its source then (the
    /// block that contains the end of a fade-out still consumes a full block of
    /// source samples);
    /// a source that ends mid-block is zero-padded and finishes the voice.
    /// The returned slice is valid until the next call; [`block`](Self::block)
    /// returns it again without advancing.
    pub fn render(&mut self, frames: usize) -> &[f32] {
        self.block.clear();
        self.block.resize(frames, 0.0);
        if self.source_ended || self.fade.is_silent() {
            return &self.block;
        }
        let read = self.source.read(&mut self.block);
        for sample in &mut self.block[..read] {
            *sample *= self.gain * self.fade.advance();
        }
        if read < frames {
            self.source_ended = true;
            self.state = PlaybackState::Finished;
            self.fade.retarget(0.0);
            self.fade.level = 0.0;
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
