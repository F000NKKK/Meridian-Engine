//! The voices of a scene rendered through one renderer.

use meridian_audio_domain::{
    BlockRenderer, DspGraph, DspNode, Emitter, Listener, PlaybackState, Voice,
};

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

    /// Interleaved channels per frame of the rendered output.
    #[must_use]
    pub fn channels(&self) -> usize {
        self.renderer.channels()
    }

    /// Starts tracking `voice` and returns its id.
    pub fn add_voice(&mut self, voice: Voice) -> VoiceId {
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

    /// Drops every finished voice and returns how many were removed.
    pub fn reap_finished(&mut self) -> usize {
        let before = self.voices.len();
        self.voices
            .retain(|(_, voice)| voice.state() != PlaybackState::Finished);
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
