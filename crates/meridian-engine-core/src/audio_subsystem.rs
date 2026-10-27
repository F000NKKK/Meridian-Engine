//! The audio state the runtime mixes each frame.

use meridian_audio_core::{Emitter, Listener, Mixer};

/// `audio-core`'s listener, emitters and gain mixer — bundled as one
/// independently-ownable/-lockable unit for the same reason
/// [`PhysicsSubsystem`](crate::PhysicsSubsystem) is split out. **One specific, opinionated audio
/// path (`Mixer::mix`'s per-channel gain model), not "the" audio
/// pipeline every consumer must fit** — see [`AudioSubsystem::mix`]'s
/// own doc comment.
#[derive(Debug)]
pub struct AudioSubsystem {
    pub listener: Listener,
    pub emitters: Vec<(Emitter, f32)>,
    pub mixer: Mixer,
}

impl AudioSubsystem {
    pub fn new(mixer: Mixer) -> Self {
        Self {
            listener: Listener::default(),
            emitters: Vec::new(),
            mixer,
        }
    }

    /// Per-channel gains for every emitter against the current listener,
    /// via `audio-core`'s `Mixer` — reads whatever `emitters`' frames are
    /// *right now*, so calling this after
    /// [`PhysicsSubsystem::step`](crate::PhysicsSubsystem::step) reflects physics-updated positions.
    ///
    /// This is one specific, opinionated audio path (`Mixer::mix`'s
    /// per-channel gain model), not "the" audio pipeline every consumer
    /// must fit. A richer consumer (`examples/magic_figures`, which
    /// needs `BinauralRenderer`'s real per-sample stereo synthesis —
    /// ITD, head-shadow filtering, its own declick stage — none of which
    /// `Mixer::mix`'s gains can express) is deliberately **not** forced
    /// through this type: bolting a second fixed pipeline (a
    /// `binaural: Option<BinauralRenderer>` field plus a matching
    /// hardcoded declick stage) onto `AudioSubsystem` alongside this one
    /// would repeat the same mistake — engine-core dictating a specific
    /// effect chain — rather than fixing it. [`Stage`](crate::Stage) is the actual,
    /// now-real fix (an app-defined `Stage` can own a `BinauralRenderer`
    /// directly); `examples/magic_figures` still doesn't route through
    /// one, though — see this module's own top-level doc for why its
    /// specific inputs (orbiting shapes' positions, the free-fly
    /// camera's pose) don't reduce to a `StageContext`-shaped `Stage`
    /// cleanly, not a gap in the mechanism itself.
    pub fn mix(&self) -> Vec<(meridian_audio_core::Channel, f32)> {
        self.mixer.mix(&self.listener, &self.emitters)
    }
}
