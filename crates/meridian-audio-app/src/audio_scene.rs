//! The voices of a scene rendered through one renderer.

use meridian_audio_domain::{BlockRenderer, DspGraph, DspNode, Emitter, Listener, Voice};

use crate::VoiceId;

/// A listener, the voices playing around it, the [`BlockRenderer`] that
/// spatializes them and an optional chain of post effects. The composition
/// root picks the renderer (speaker-gain `Mixer` or `BinauralRenderer`)
/// and the effects; this type dictates neither. Plain data, no lock.
pub struct AudioScene {
    /// Where the scene is heard from.
    pub listener: Listener,
    voices: Vec<(VoiceId, Voice)>,
    next_id: u64,
    max_voices: usize,
    renderer: Box<dyn BlockRenderer>,
    post: DspGraph,
}

impl AudioScene {
    /// An empty scene rendered through `renderer`.
    #[must_use]
    pub fn new(renderer: impl BlockRenderer + 'static) -> Self {
        Self {
            listener: Listener::default(),
            voices: Vec::new(),
            next_id: 0,
            max_voices: usize::MAX,
            renderer: Box::new(renderer),
            post: DspGraph::new(),
        }
    }

    /// Appends `effect` to the chain applied to every rendered block.
    #[must_use]
    pub fn with_post_effect(mut self, effect: impl DspNode + 'static) -> Self {
        self.post.push(effect);
        self
    }

    /// Caps the scene at `max_voices` voices (at least one). Adding a voice
    /// to a full scene first drops voices that are done, then steals the
    /// oldest voice, so a burst of one-shots never grows the scene without
    /// bound.
    #[must_use]
    pub fn with_max_voices(mut self, max_voices: usize) -> Self {
        self.max_voices = max_voices.max(1);
        self
    }

    /// Interleaved channels per frame of the rendered output.
    #[must_use]
    pub fn channels(&self) -> usize {
        self.renderer.channels()
    }

    /// Starts tracking `voice` and returns its id. In a scene at its voice
    /// limit this drops done voices, then the oldest voice, to make room.
    pub fn add_voice(&mut self, voice: Voice) -> VoiceId {
        if self.voices.len() >= self.max_voices {
            self.reap_finished();
        }
        while self.voices.len() >= self.max_voices {
            self.voices.remove(0);
        }
        let id = VoiceId::new(self.next_id);
        self.next_id += 1;
        self.voices.push((id, voice));
        id
    }

    /// The voice with `id`, if it is still in the scene.
    pub fn voice_mut(&mut self, id: VoiceId) -> Option<&mut Voice> {
        self.voices
            .iter_mut()
            .find(|(voice_id, _)| *voice_id == id)
            .map(|(_, voice)| voice)
    }

    /// Moves the emitter of voice `id`; `false` if the voice is gone.
    pub fn set_emitter(&mut self, id: VoiceId, emitter: Emitter) -> bool {
        self.voice_mut(id)
            .map(|voice| voice.emitter = emitter)
            .is_some()
    }

    /// Drops every voice that is done (finished and no longer audible, so a
    /// fade-out is allowed to play out) and returns how many were removed.
    pub fn reap_finished(&mut self) -> usize {
        let before = self.voices.len();
        self.voices.retain(|(_, voice)| !voice.is_done());
        before - self.voices.len()
    }

    /// Number of voices in the scene, finished ones included.
    #[must_use]
    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    /// Renders the next `out.len() / channels()` frames of every voice,
    /// spatialized for the current listener, into `out` (overwritten), then
    /// runs the post effects over it. Every voice advances by the same
    /// number of frames, in the order they were added.
    pub fn render_block(&mut self, out: &mut [f32]) {
        let frames = out.len() / self.renderer.channels().max(1);
        for (_, voice) in &mut self.voices {
            voice.render(frames);
        }
        let sources: Vec<(Emitter, &[f32])> = self
            .voices
            .iter()
            .map(|(_, voice)| (voice.emitter, voice.block()))
            .collect();
        self.renderer.render(&self.listener, &sources, out);
        self.post.process(out);
    }
}

impl std::fmt::Debug for AudioScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioScene")
            .field("voices", &self.voices.len())
            .field("channels", &self.renderer.channels())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use meridian_audio_domain::{Declicker, Gain, MemorySource, Mixer, Repeat, SpeakerLayout};
    use meridian_gac::{Motor3, Vec3};

    use super::*;

    fn right_of_listener() -> Emitter {
        Emitter {
            frame: Motor3::translation(Vec3::new(0.0, 0.0, 0.5)),
        }
    }

    fn stereo_scene() -> AudioScene {
        AudioScene::new(Mixer::new(SpeakerLayout::stereo_headphones()))
    }

    #[test]
    fn a_voice_right_of_the_listener_plays_in_the_right_channel() {
        let mut scene = stereo_scene();
        scene.add_voice(Voice::new(
            MemorySource::new(vec![1.0; 8], Repeat::Loop),
            right_of_listener(),
        ));

        let mut out = [0.0f32; 8];
        scene.render_block(&mut out);

        assert!(out.chunks_exact(2).all(|f| f[1] > 0.5 && f[0].abs() < 1e-3));
    }

    #[test]
    fn moving_the_emitter_moves_the_sound() {
        let mut scene = stereo_scene();
        let id = scene.add_voice(Voice::new(
            MemorySource::new(vec![1.0; 4], Repeat::Loop),
            right_of_listener(),
        ));
        let left = Emitter {
            frame: Motor3::translation(Vec3::new(0.0, 0.0, -0.5)),
        };
        assert!(scene.set_emitter(id, left));

        let mut out = [0.0f32; 4];
        scene.render_block(&mut out);

        assert!(out[0] > 0.5 && out[1].abs() < 1e-3);
    }

    #[test]
    fn finished_voices_are_reaped_and_their_ids_go_stale() {
        let mut scene = stereo_scene();
        let id = scene.add_voice(Voice::new(
            MemorySource::new(vec![1.0], Repeat::Once),
            right_of_listener(),
        ));

        scene.render_block(&mut [0.0; 4]);

        assert_eq!(scene.reap_finished(), 1);
        assert_eq!(scene.voice_count(), 0);
        assert!(!scene.set_emitter(id, right_of_listener()));
    }

    fn looping_voice() -> Voice {
        Voice::new(
            MemorySource::new(vec![1.0; 4], Repeat::Loop),
            right_of_listener(),
        )
    }

    #[test]
    fn a_full_scene_steals_the_oldest_voice() {
        let mut scene = stereo_scene().with_max_voices(2);
        let oldest = scene.add_voice(looping_voice());
        let middle = scene.add_voice(looping_voice());

        let newest = scene.add_voice(looping_voice());

        assert_eq!(scene.voice_count(), 2);
        assert!(scene.voice_mut(oldest).is_none());
        assert!(scene.voice_mut(middle).is_some());
        assert!(scene.voice_mut(newest).is_some());
    }

    #[test]
    fn a_full_scene_prefers_dropping_done_voices_to_stealing() {
        let mut scene = stereo_scene().with_max_voices(2);
        let oldest = scene.add_voice(looping_voice());
        let stopped = scene.add_voice(looping_voice());
        scene.voice_mut(stopped).unwrap().stop();

        scene.add_voice(looping_voice());

        assert!(scene.voice_mut(oldest).is_some(), "the live voice survives");
        assert!(scene.voice_mut(stopped).is_none());
    }

    #[test]
    fn a_fading_out_voice_is_kept_until_it_is_silent() {
        let mut scene = stereo_scene();
        let id = scene.add_voice(looping_voice().with_fade(2));
        scene.render_block(&mut [0.0; 4]);
        scene.voice_mut(id).unwrap().stop();

        assert_eq!(scene.reap_finished(), 0, "the fade-out is still audible");
        scene.render_block(&mut [0.0; 6]);
        assert_eq!(scene.reap_finished(), 1);
    }

    #[test]
    fn post_effects_run_over_the_rendered_block() {
        let mut scene = stereo_scene().with_post_effect(Gain(0.5));
        scene.add_voice(Voice::new(
            MemorySource::new(vec![1.0; 2], Repeat::Loop),
            right_of_listener(),
        ));
        let mut plain = stereo_scene();
        plain.add_voice(Voice::new(
            MemorySource::new(vec![1.0; 2], Repeat::Loop),
            right_of_listener(),
        ));

        let (mut halved, mut full) = ([0.0f32; 4], [0.0f32; 4]);
        scene.render_block(&mut halved);
        plain.render_block(&mut full);

        assert!((halved[1] - full[1] * 0.5).abs() < 1e-6);
    }

    #[test]
    fn an_empty_scene_renders_silence() {
        let mut scene = stereo_scene().with_post_effect(Declicker::new(2));
        let mut out = [1.0f32; 6];
        scene.render_block(&mut out);

        assert!(out.iter().all(|s| s.abs() < 1e-6));
    }
}
