//! The audio state the runtime mixes each frame.

use meridian_audio_domain::{Emitter, Listener, Mixer};

/// `audio-domain`'s listener, emitters and gain mixer, bundled as one
/// plain-data unit the application owns; no lock lives here. **One
/// specific, opinionated audio path (`Mixer::mix`'s per-channel gain
/// model), not "the" audio pipeline every consumer must fit** — see
/// [`AudioSubsystem::mix`]'s own doc comment.
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
    /// via `audio-domain`'s `Mixer` — reads whatever `emitters`' frames are
    /// *right now*, so calling this after a physics step reflects
    /// physics-updated positions.
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
    /// would repeat the same mistake — runtime-app dictating a specific
    /// effect chain — rather than fixing it. An application-defined
    /// runtime stage can own a `BinauralRenderer` directly instead.
    pub fn mix(&self) -> Vec<(meridian_audio_domain::Channel, f32)> {
        self.mixer.mix(&self.listener, &self.emitters)
    }
}

#[cfg(test)]
mod tests {
    use meridian_audio_domain::{AttenuationModel, Channel, SpeakerLayout};
    use meridian_gac::{Motor3, Vec3};

    use super::*;

    #[test]
    fn mixes_audio_from_current_emitter_positions() {
        let mut audio = AudioSubsystem::new(
            Mixer::new(SpeakerLayout::stereo_headphones()).with_attenuation(AttenuationModel {
                reference_distance: 1000.0,
                rolloff: 1.0,
                max_distance: 1000.0,
            }),
        );
        audio.listener = Listener {
            frame: Motor3::identity(),
        };
        // Local +Z is "right" per audio-domain's listener convention.
        audio.emitters.push((
            Emitter {
                frame: Motor3::translation(Vec3::new(0.0, 0.0, 5.0)),
            },
            1.0,
        ));

        let gains = audio.mix();
        let gain_of = |channel: Channel| {
            gains
                .iter()
                .find(|(c, _)| *c == channel)
                .map_or(0.0, |(_, g)| *g)
        };
        assert!(gain_of(Channel::Right) > 0.99);
        assert!(gain_of(Channel::Left) < 1e-3);
    }
}
