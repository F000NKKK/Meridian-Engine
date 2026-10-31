//! Renders emitters into speaker channels.

use crate::spatial_gains::spatial_gains_into;
use crate::{AttenuationModel, Channel, Emitter, Listener, SpeakerLayout};

/// Layouts up to this many speakers keep per-emitter gains on the stack.
const STACK_CHANNELS: usize = 16;

/// Mixes active emitters down to the output stream: one gain-weighted
/// sample per output channel, summed across every `(Emitter, sample)`
/// pair given to [`Mixer::mix`].
#[derive(Debug, Clone)]
pub struct Mixer {
    /// Output speakers used for spatial mixing.
    pub layout: SpeakerLayout,
    /// Distance attenuation applied to each source.
    pub attenuation: AttenuationModel,
}

impl Mixer {
    /// Creates a mixer with default attenuation.
    pub fn new(layout: SpeakerLayout) -> Self {
        Self {
            layout,
            attenuation: AttenuationModel::default(),
        }
    }

    /// Sets the distance attenuation model.
    pub fn with_attenuation(mut self, attenuation: AttenuationModel) -> Self {
        self.attenuation = attenuation;
        self
    }

    /// `output[channel] = sum_i(sample_i * spatial_gains(listener, emitter_i)[channel])`.
    pub fn mix(&self, listener: &Listener, emitters: &[(Emitter, f32)]) -> Vec<(Channel, f32)> {
        let mut sums = vec![0.0; self.layout.speakers.len()];
        self.mix_into(listener, emitters, &mut sums);
        self.layout
            .speakers
            .iter()
            .zip(sums)
            .map(|(speaker, sum)| (speaker.channel, sum))
            .collect()
    }

    /// [`mix`](Self::mix) without allocating (for layouts of up to 16
    /// speakers): overwrites `output[i]` with the sum for
    /// `layout.speakers[i]`.
    pub fn mix_into(&self, listener: &Listener, emitters: &[(Emitter, f32)], output: &mut [f32]) {
        output.fill(0.0);
        self.with_gain_scratch(|scratch| {
            for (emitter, sample) in emitters {
                self.emitter_gains(listener, *emitter, scratch);
                for (out, gain) in output.iter_mut().zip(scratch.iter()) {
                    *out += gain * sample;
                }
            }
        });
    }

    /// Renders `frames` output frames as interleaved samples in this
    /// layout's speaker order — the format `audio-cpal`'s
    /// `AudioStream::push_samples` (any `AudioSink` of `meridian-audio-app`) consumes.
    ///
    /// The block-sized counterpart of [`mix`](Self::mix): each emitter's
    /// spatial gains are computed once for the whole block (emitter and
    /// listener frames don't move mid-frame; per-sample re-panning would
    /// recompute the identical gains `frames` times), then every mono
    /// source sample is gain-weighted into each output channel. A source
    /// shorter than `frames` is padded with silence.
    pub fn render_interleaved(
        &self,
        listener: &Listener,
        sources: &[(Emitter, &[f32])],
        frames: usize,
    ) -> Vec<f32> {
        let mut out = vec![0.0f32; frames * self.layout.speakers.len()];
        self.render_interleaved_into(listener, sources, &mut out);
        out
    }

    /// [`render_interleaved`](Self::render_interleaved) into a caller-owned
    /// buffer, so a loop can reuse one block buffer: `out` holds
    /// `out.len() / channels` frames and is overwritten.
    pub fn render_interleaved_into(
        &self,
        listener: &Listener,
        sources: &[(Emitter, &[f32])],
        out: &mut [f32],
    ) {
        out.fill(0.0);
        let channels = self.layout.speakers.len();
        if channels == 0 {
            return;
        }
        self.with_gain_scratch(|scratch| {
            for (emitter, samples) in sources {
                self.emitter_gains(listener, *emitter, scratch);
                accumulate_source(out, channels, scratch, samples);
            }
        });
    }

    fn emitter_gains(&self, listener: &Listener, emitter: Emitter, gains: &mut [f32]) {
        spatial_gains_into(listener, emitter, &self.layout, self.attenuation, gains);
    }

    /// Runs `body` with a per-emitter gain scratch of one slot per speaker:
    /// on the stack for ordinary layouts, on the heap beyond 16 speakers.
    fn with_gain_scratch(&self, body: impl FnOnce(&mut [f32])) {
        let channels = self.layout.speakers.len();
        let mut stack = [0.0f32; STACK_CHANNELS];
        if channels <= STACK_CHANNELS {
            body(&mut stack[..channels]);
        } else {
            body(&mut vec![0.0f32; channels]);
        }
    }
}

/// Adds a mono `samples` block into interleaved `out`, weighted per channel.
fn accumulate_source(out: &mut [f32], channels: usize, gains: &[f32], samples: &[f32]) {
    for (frame, sample) in out.chunks_exact_mut(channels).zip(samples) {
        for (slot, gain) in frame.iter_mut().zip(gains) {
            *slot += gain * sample;
        }
    }
}
