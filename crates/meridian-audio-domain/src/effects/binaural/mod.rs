//! Binaural ("ear effect") headphone spatializer: ITD, head-shadow
//! filtering, rear damping and distance attenuation, with per-block
//! parameter ramping. See [`BinauralRenderer`].

use super::AcousticMedium;
use crate::{AttenuationModel, Emitter, Listener};

/// Tunable parameters of the binaural model — nothing about how the
/// listener's head or the filters behave is hard-coded; the propagation
/// medium itself is a separate knob ([`AcousticMedium`] on
/// [`BinauralRenderer::with_medium`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BinauralConfig {
    /// Distance between the ears in meters — with the medium's speed of
    /// sound, sets the maximum interaural time difference.
    pub ear_separation_m: f32,
    /// Head-shadow cutoff for the fully *unshadowed* ear, Hz (clamped
    /// below Nyquist at render time). Near-transparent by default.
    pub shadow_open_cutoff_hz: f32,
    /// Head-shadow cutoff for the fully *shadowed* ear, Hz — what the
    /// far ear is left with when the source sits hard to one side.
    pub shadow_closed_cutoff_hz: f32,
    /// Fraction of gain removed directly behind the listener (`0.0` =
    /// rear as loud as front, `0.25` = 25% quieter).
    pub rear_gain_reduction: f32,
    /// Fraction of filter cutoff removed directly behind the listener —
    /// how much duller the rear sounds.
    pub rear_cutoff_reduction: f32,
}

impl Default for BinauralConfig {
    fn default() -> Self {
        Self {
            ear_separation_m: 0.21,
            shadow_open_cutoff_hz: 18_000.0,
            shadow_closed_cutoff_hz: 900.0,
            rear_gain_reduction: 0.25,
            rear_cutoff_reduction: 0.55,
        }
    }
}

/// Per-ear render targets for one source at one listener pose. Ear index
/// 0 is left, 1 is right — the interleave order of a stereo stream.
#[derive(Debug, Clone, Copy)]
struct EarTargets {
    gain: [f32; 2],
    /// Delay in (fractional) samples — the interaural time difference.
    delay: [f32; 2],
    /// One-pole low-pass coefficient (`y += alpha * (x - y)`): small =
    /// heavy muffling, near 1 = transparent.
    alpha: [f32; 2],
}

/// Recent input history for one source: a ring the two ears read at
/// different (fractional) delays.
#[derive(Debug)]
struct HistoryRing {
    buf: Vec<f32>,
    pos: usize,
}

impl HistoryRing {
    fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len],
            pos: 0,
        }
    }

    fn write(&mut self, sample: f32) {
        self.pos = (self.pos + 1) % self.buf.len();
        self.buf[self.pos] = sample;
    }

    /// The sample `delay` samples ago, linearly interpolated between the
    /// two neighbors for fractional delays.
    fn read(&self, delay: f32) -> f32 {
        let len = self.buf.len();
        let whole = delay.floor() as usize;
        let frac = delay - delay.floor();
        let newer = self.buf[(self.pos + len - whole.min(len - 1)) % len];
        let older = self.buf[(self.pos + len - (whole + 1).min(len - 1)) % len];
        newer * (1.0 - frac) + older * frac
    }
}

#[derive(Debug)]
struct BinauralSourceState {
    history: HistoryRing,
    /// One-pole low-pass filter state per ear.
    lpf: [f32; 2],
    /// Previous block's targets — the ramp start, so gains/delays/filters
    /// glide instead of stepping at block boundaries (stepping is audible
    /// as zipper noise/crackle when a gain fades).
    prev: Option<EarTargets>,
}

impl BinauralSourceState {
    fn new(max_itd_samples: f32) -> Self {
        // Enough history for the medium's worst-case ITD plus headroom
        // (slow media like air need ~30 samples at 48 kHz; water far
        // fewer — sound crosses the head faster).
        let capacity = (max_itd_samples as usize + 8).max(64);
        Self {
            history: HistoryRing::new(capacity),
            lpf: [0.0; 2],
            prev: None,
        }
    }
}

/// Headphone ("ear effect") spatializer — the stereo-only, stateful
/// upgrade over [`Mixer::render_interleaved`](crate::Mixer::render_interleaved)'s amplitude panning.
/// Models, per source:
///
/// - **ITD** (interaural time difference): the far ear hears the source
///   up to ~0.6 ms later — a fractional-sample delay line per ear.
/// - **Head shadow**: high frequencies don't bend around the head, so
///   the far ear gets a one-pole low-pass whose cutoff drops as the
///   source moves to the opposite side — low frequencies still arrive,
///   highs don't. The near ear stays (almost) transparent.
/// - **Behind the listener**: mildly quieter and duller (lower cutoff on
///   both ears) — the pinna-less approximation of a rear source; unlike
///   [`fold_to_front_hemisphere`](crate::fold_to_front_hemisphere) the rear is *not* folded onto the
///   front, so front and back genuinely differ.
/// - **Distance**: the same [`AttenuationModel`] as the mixer, plus the
///   medium's high-frequency absorption — far sources sound duller, and
///   how much duller depends on what the sound travels through.
///
/// The propagation medium ([`AcousticMedium`] — air at sea level by
/// default, altitude/water presets, or fully custom) sets the speed of
/// sound (and with it the ITD) and the absorption; head geometry and
/// every filter bound live in [`BinauralConfig`]. Nothing is a
/// hard-coded constant.
///
/// All parameters ramp linearly across each rendered block from the
/// previous block's values, so a moving listener produces smooth gain/
/// delay glides instead of per-block steps (audible as crackle).
///
/// Stateful: keep one renderer alive across blocks and pass sources in a
/// stable order (state is per source index). Output is interleaved
/// stereo `[L, R]` — the layout [`SpeakerLayout::stereo_headphones`](crate::SpeakerLayout::stereo_headphones)
/// describes, ready for `AudioSink::push_interleaved` (`meridian-audio-app`).
#[derive(Debug)]
pub struct BinauralRenderer {
    sample_rate: u32,
    pub attenuation: AttenuationModel,
    pub medium: AcousticMedium,
    pub config: BinauralConfig,
    sources: Vec<BinauralSourceState>,
}

impl BinauralRenderer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            attenuation: AttenuationModel::default(),
            medium: AcousticMedium::default(),
            config: BinauralConfig::default(),
            sources: Vec::new(),
        }
    }

    pub fn with_attenuation(mut self, attenuation: AttenuationModel) -> Self {
        self.attenuation = attenuation;
        self
    }

    /// Sets the propagation medium. Existing per-source delay lines are
    /// rebuilt on the next render if the new medium needs more history.
    pub fn with_medium(mut self, medium: AcousticMedium) -> Self {
        self.medium = medium;
        self.sources.clear();
        self
    }

    pub fn with_config(mut self, config: BinauralConfig) -> Self {
        self.config = config;
        self.sources.clear();
        self
    }

    /// The medium's worst-case interaural delay, in samples.
    fn max_itd_samples(&self) -> f32 {
        self.config.ear_separation_m / self.medium.speed_of_sound_m_s.max(1.0)
            * self.sample_rate as f32
    }

    /// One-pole coefficient for a cutoff frequency at this sample rate.
    fn alpha_for_cutoff(&self, cutoff_hz: f32) -> f32 {
        1.0 - (-std::f32::consts::TAU * cutoff_hz / self.sample_rate as f32).exp()
    }

    /// Where `emitter` sits relative to `listener`, translated into
    /// per-ear gain/delay/filter targets.
    fn targets(&self, listener: &Listener, emitter: &Emitter) -> EarTargets {
        let local = listener.frame.inverse().transform_point(emitter.position());
        let distance = local.length();
        let distance_gain = self.attenuation.gain(distance);

        // Horizontal plane only, like the rest of this crate: forward
        // +X, right +Z (see the module doc).
        let planar = (local.x * local.x + local.z * local.z).sqrt();
        let (sin_az, cos_az) = if planar > 1e-6 {
            (local.z / planar, local.x / planar)
        } else {
            (0.0, 1.0) // directly above/below/at the listener: treat as front
        };

        // Constant-power level difference over the full circle —
        // sin(azimuth) is the lateral component, identical for a source
        // at 30° and its mirror at 150°, which is correct: ILD/ITD are
        // front/back-symmetric; the rear term below is what differs.
        let gain_l = ((1.0 - sin_az) / 2.0).sqrt();
        let gain_r = f32::midpoint(1.0, sin_az).sqrt();

        // Behind: mildly quieter and duller on both ears.
        let rear = (-cos_az).max(0.0);
        let rear_gain = 1.0 - self.config.rear_gain_reduction * rear;
        let rear_cutoff_scale = 1.0 - self.config.rear_cutoff_reduction * rear;

        // The medium absorbs high frequencies with distance —
        // exponential cutoff decay, stronger in air than in water.
        let absorption_scale = (-self.medium.high_freq_absorption_per_m * distance).exp();

        // ITD: a source to the right (+sin_az) reaches the left ear
        // later. The medium's speed of sound sets how much later.
        let itd_samples = self.max_itd_samples();
        let delay_l = (sin_az * itd_samples).max(0.0);
        let delay_r = (-sin_az * itd_samples).max(0.0);

        // Head shadow: the ear opposite the source loses highs. Cutoff
        // slides from the open bound toward the closed bound as the
        // source moves fully to the other side.
        let shadow_l = sin_az.max(0.0); // source right -> left ear shadowed
        let shadow_r = (-sin_az).max(0.0);
        let cutoff = |shadow: f32| {
            let open = self
                .config
                .shadow_open_cutoff_hz
                .min(self.sample_rate as f32 * 0.45);
            let closed = self.config.shadow_closed_cutoff_hz.min(open);
            (open + (closed - open) * shadow) * rear_cutoff_scale * absorption_scale
        };

        EarTargets {
            gain: [
                gain_l * rear_gain * distance_gain,
                gain_r * rear_gain * distance_gain,
            ],
            delay: [delay_l, delay_r],
            alpha: [
                self.alpha_for_cutoff(cutoff(shadow_l)),
                self.alpha_for_cutoff(cutoff(shadow_r)),
            ],
        }
    }

    /// Renders `frames` interleaved stereo samples for `sources` (mono
    /// sample blocks, padded with silence when shorter than `frames`) as
    /// heard by `listener`. Call once per block with sources in a stable
    /// order — delay lines, filter state and parameter ramps continue
    /// across calls.
    pub fn render(
        &mut self,
        listener: &Listener,
        sources: &[(Emitter, &[f32])],
        frames: usize,
    ) -> Vec<f32> {
        let mut out = vec![0.0f32; frames * 2];
        self.render_into(listener, sources, &mut out);
        out
    }

    /// [`render`](Self::render) into a caller-owned stereo buffer (`out`
    /// holds `out.len() / 2` frames and is overwritten), so a loop can
    /// reuse one block buffer.
    pub fn render_into(
        &mut self,
        listener: &Listener,
        sources: &[(Emitter, &[f32])],
        out: &mut [f32],
    ) {
        let frames = out.len() / 2;
        out.fill(0.0);
        while self.sources.len() < sources.len() {
            self.sources
                .push(BinauralSourceState::new(self.max_itd_samples()));
        }

        for (index, (emitter, samples)) in sources.iter().enumerate() {
            let target = self.targets(listener, emitter);
            let state = &mut self.sources[index];
            let start = state.prev.unwrap_or(target);

            for frame in 0..frames {
                let x = samples.get(frame).copied().unwrap_or(0.0);
                state.history.write(x);
                let k = if frames > 1 {
                    frame as f32 / (frames - 1) as f32
                } else {
                    1.0
                };
                for ear in 0..2 {
                    let gain = start.gain[ear] + (target.gain[ear] - start.gain[ear]) * k;
                    let delay = start.delay[ear] + (target.delay[ear] - start.delay[ear]) * k;
                    let alpha = start.alpha[ear] + (target.alpha[ear] - start.alpha[ear]) * k;
                    let delayed = state.history.read(delay);
                    state.lpf[ear] += alpha * (delayed - state.lpf[ear]);
                    out[frame * 2 + ear] += state.lpf[ear] * gain;
                }
            }
            state.prev = Some(target);
        }
    }
}

#[cfg(test)]
mod tests;
