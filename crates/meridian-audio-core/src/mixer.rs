//! Renders emitters into speaker channels.

use crate::{AttenuationModel, Channel, Emitter, Listener, SpeakerLayout, spatial_gains};

/// Mixes active emitters down to the output stream: one gain-weighted
/// sample per output channel, summed across every `(Emitter, sample)`
/// pair given to [`Mixer::mix`].
#[derive(Debug, Clone)]
pub struct Mixer {
    pub layout: SpeakerLayout,
    pub attenuation: AttenuationModel,
}

impl Mixer {
    pub fn new(layout: SpeakerLayout) -> Self {
        Self {
            layout,
            attenuation: AttenuationModel::default(),
        }
    }

    pub fn with_attenuation(mut self, attenuation: AttenuationModel) -> Self {
        self.attenuation = attenuation;
        self
    }

    /// `output[channel] = sum_i(sample_i * spatial_gains(listener, emitter_i)[channel])`.
    pub fn mix(&self, listener: &Listener, emitters: &[(Emitter, f32)]) -> Vec<(Channel, f32)> {
        let mut output: Vec<(Channel, f32)> = self
            .layout
            .speakers
            .iter()
            .map(|s| (s.channel, 0.0))
            .collect();
        for (emitter, sample) in emitters {
            let gains = spatial_gains(listener, emitter, &self.layout, &self.attenuation);
            for (i, (_, gain)) in gains.into_iter().enumerate() {
                output[i].1 += gain * sample;
            }
        }
        output
    }

    /// Renders `frames` output frames as interleaved samples in this
    /// layout's speaker order — the format `audio-driver`'s
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
        let channels = self.layout.speakers.len();
        let mut out = vec![0.0f32; frames * channels];
        for (emitter, samples) in sources {
            let gains = spatial_gains(listener, emitter, &self.layout, &self.attenuation);
            for (frame, sample) in samples.iter().take(frames).copied().enumerate() {
                for (ch, (_, gain)) in gains.iter().enumerate() {
                    out[frame * channels + ch] += gain * sample;
                }
            }
        }
        out
    }
}
