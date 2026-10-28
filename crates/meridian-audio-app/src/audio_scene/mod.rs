//! The voices of a scene rendered through one renderer.

use meridian_audio_domain::{BlockRenderer, DspGraph, DspNode, Emitter, Listener, Voice};

use crate::{BusId, VoiceId};

/// A listener, the voices playing around it, the [`BlockRenderer`] that
/// spatializes them and an optional chain of post effects. The composition
/// root picks the renderer (speaker-gain `Mixer` or `BinauralRenderer`)
/// and the effects; this type dictates neither. Voices can be grouped on
/// buses ([`add_bus`](Self::add_bus)) whose gain scales every voice on them;
/// [`BusId::MASTER`] scales the whole mix. Plain data, no lock.
pub struct AudioScene {
    /// Where the scene is heard from.
    pub listener: Listener,
    voices: Vec<(VoiceId, BusId, Voice)>,
    /// Linear gain per bus; index 0 is [`BusId::MASTER`].
    bus_gains: Vec<f32>,
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
            bus_gains: vec![1.0],
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
    /// voice with the lowest [`priority`](Voice::priority) (the oldest among
    /// equals), so a burst of one-shots never grows the scene without bound.
    /// A new voice of lower priority than every voice in the scene is
    /// dropped instead of stealing.
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
    /// limit this drops done voices, then the lowest-priority voice, to make
    /// room; if every voice outranks the new one, the new voice is dropped and
    /// its id resolves to nothing.
    pub fn add_voice(&mut self, voice: Voice) -> VoiceId {
        self.add_voice_to(BusId::MASTER, voice)
    }

    /// Like [`add_voice`](Self::add_voice), mixing the voice through `bus`
    /// (an unknown bus means [`BusId::MASTER`]).
    pub fn add_voice_to(&mut self, bus: BusId, voice: Voice) -> VoiceId {
        let bus = if bus.index() < self.bus_gains.len() {
            bus
        } else {
            BusId::MASTER
        };
        let id = VoiceId::new(self.next_id);
        self.next_id += 1;
        if self.voices.len() >= self.max_voices {
            self.reap_finished();
        }
        while self.voices.len() >= self.max_voices {
            let Some(victim) = self.steal_candidate() else {
                return id;
            };
            if self.voices[victim].2.priority > voice.priority {
                return id;
            }
            self.voices.remove(victim);
        }
        self.voices.push((id, bus, voice));
        id
    }

    /// Index of the lowest-priority voice, the oldest among equals.
    fn steal_candidate(&self) -> Option<usize> {
        self.voices
            .iter()
            .enumerate()
            .min_by_key(|(index, (_, _, voice))| (voice.priority, *index))
            .map(|(index, _)| index)
    }

    /// The voice with `id`, if it is still in the scene.
    pub fn voice_mut(&mut self, id: VoiceId) -> Option<&mut Voice> {
        self.voices
            .iter_mut()
            .find(|(voice_id, _, _)| *voice_id == id)
            .map(|(_, _, voice)| voice)
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
        self.voices.retain(|(_, _, voice)| !voice.is_done());
        before - self.voices.len()
    }

    /// Creates a bus with unit gain; voices join it through
    /// [`add_voice_to`](Self::add_voice_to).
    pub fn add_bus(&mut self) -> BusId {
        self.bus_gains.push(1.0);
        BusId::new(self.bus_gains.len() - 1)
    }

    /// Sets the linear gain of `bus` (negative and non-finite values mean
    /// silence); `false` if the bus does not exist. The gain applies from the
    /// next block, with no ramp: pair it with voice fades for audible changes.
    pub fn set_bus_gain(&mut self, bus: BusId, gain: f32) -> bool {
        let gain = if gain.is_finite() { gain.max(0.0) } else { 0.0 };
        self.bus_gains
            .get_mut(bus.index())
            .map(|slot| *slot = gain)
            .is_some()
    }

    /// The gain of `bus`, if it exists.
    #[must_use]
    pub fn bus_gain(&self, bus: BusId) -> Option<f32> {
        self.bus_gains.get(bus.index()).copied()
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
        for (_, bus, voice) in &mut self.voices {
            // The master gain is applied once, to the finished mix.
            let scale = if *bus == BusId::MASTER {
                1.0
            } else {
                self.bus_gains[bus.index()]
            };
            voice.render_scaled(frames, scale);
        }
        let sources: Vec<(Emitter, &[f32])> = self
            .voices
            .iter()
            .map(|(_, _, voice)| (voice.emitter, voice.block()))
            .collect();
        self.renderer.render(&self.listener, &sources, out);
        let master = self.bus_gains[BusId::MASTER.index()];
        for sample in out.iter_mut() {
            *sample *= master;
        }
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
mod tests;
