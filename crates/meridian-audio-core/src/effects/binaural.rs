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
/// upgrade over [`Mixer::render_interleaved`]'s amplitude panning.
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
///   [`fold_to_front_hemisphere`] the rear is *not* folded onto the
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
/// stereo `[L, R]` — the layout [`SpeakerLayout::stereo_headphones`]
/// describes, ready for [`AudioOutput::push_interleaved`].
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
        let gain_r = ((1.0 + sin_az) / 2.0).sqrt();

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
        while self.sources.len() < sources.len() {
            self.sources
                .push(BinauralSourceState::new(self.max_itd_samples()));
        }

        let mut out = vec![0.0f32; frames * 2];
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
        out
    }
}

#[cfg(test)]
mod binaural_tests {
    use super::*;
    use meridian_gac_core::{Motor3, Vec3};

    const RATE: u32 = 48_000;

    fn listener() -> Listener {
        Listener {
            frame: Motor3::identity(),
        }
    }

    fn emitter_at(position: Vec3) -> Emitter {
        Emitter {
            frame: Motor3::translation(position),
        }
    }

    /// No distance attenuation — directional behavior only.
    fn renderer() -> BinauralRenderer {
        BinauralRenderer::new(RATE).with_attenuation(AttenuationModel {
            reference_distance: 1000.0,
            rolloff: 1.0,
            max_distance: 1000.0,
        })
    }

    fn rms(samples: impl Iterator<Item = f32>) -> f32 {
        let (sum, n) = samples.fold((0.0f32, 0usize), |(s, n), x| (s + x * x, n + 1));
        (sum / n.max(1) as f32).sqrt()
    }

    fn left(out: &[f32]) -> impl Iterator<Item = f32> + '_ {
        out.chunks_exact(2).map(|f| f[0])
    }

    fn right(out: &[f32]) -> impl Iterator<Item = f32> + '_ {
        out.chunks_exact(2).map(|f| f[1])
    }

    #[test]
    fn source_on_the_right_is_louder_in_the_right_ear() {
        let mut r = renderer();
        let source = vec![0.5f32; 2048];
        let out = r.render(
            &listener(),
            &[(emitter_at(Vec3::new(0.0, 0.0, 3.0)), &source)],
            2048,
        );
        assert!(rms(right(&out)) > 4.0 * rms(left(&out)));
    }

    #[test]
    fn source_on_the_right_reaches_the_left_ear_later() {
        let mut r = renderer();
        // An impulse after warmup silence; find each ear's peak time.
        let mut source = vec![0.0f32; 256];
        source[64] = 1.0;
        // ~70 deg right: strongly lateral but neither ear is muted (a
        // hard-side source zeroes the far ear's gain, making its peak
        // index meaningless).
        let out = r.render(
            &listener(),
            &[(emitter_at(Vec3::new(1.03, 0.0, 2.82)), &source)],
            256,
        );
        let peak_index = |it: &mut dyn Iterator<Item = f32>| {
            it.enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(i, _)| i)
                .unwrap()
        };
        let left_peak = peak_index(&mut left(&out));
        let right_peak = peak_index(&mut right(&out));
        // Full-right ITD at 48 kHz is ~29 samples.
        assert!(
            left_peak >= right_peak + 20,
            "left peak {left_peak} should trail right peak {right_peak}"
        );
    }

    #[test]
    fn behind_is_quieter_than_in_front() {
        let source = vec![0.5f32; 2048];
        let mut front_renderer = renderer();
        let front = front_renderer.render(
            &listener(),
            &[(emitter_at(Vec3::new(3.0, 0.0, 0.0)), &source)],
            2048,
        );
        let mut rear_renderer = renderer();
        let rear = rear_renderer.render(
            &listener(),
            &[(emitter_at(Vec3::new(-3.0, 0.0, 0.0)), &source)],
            2048,
        );
        let total = |out: &[f32]| rms(out.iter().copied());
        assert!(
            total(&rear) < 0.85 * total(&front),
            "rear {} vs front {}",
            total(&rear),
            total(&front)
        );
    }

    #[test]
    fn shadowed_ear_loses_high_frequencies_more_than_the_near_ear() {
        // Nyquist-rate alternation is the highest frequency the stream
        // can carry; compare each ear's response to it against its
        // response to a constant, per ear — the far (shadowed) ear must
        // pass proportionally far less of the high frequency.
        let high: Vec<f32> = (0..4096)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let low = vec![0.5f32; 4096];
        let position = Vec3::new(1.03, 0.0, 2.82); // ~70° right — left ear shadowed but not silent

        let mut r_high = renderer();
        let out_high = r_high.render(&listener(), &[(emitter_at(position), &high)], 4096);
        let mut r_low = renderer();
        let out_low = r_low.render(&listener(), &[(emitter_at(position), &low)], 4096);

        let left_ratio = rms(left(&out_high)) / rms(left(&out_low));
        let right_ratio = rms(right(&out_high)) / rms(right(&out_low));
        assert!(
            left_ratio < 0.5 * right_ratio,
            "shadowed left ratio {left_ratio} vs near right ratio {right_ratio}"
        );
    }

    #[test]
    fn consecutive_blocks_with_a_pose_jump_stay_smooth() {
        // Rotate the listener 180° between blocks — the gain change per
        // output sample must stay tiny (ramped), never a step. A step is
        // the crackle/zipper artifact this renderer exists to avoid.
        let source: Vec<f32> = (0..2400)
            .map(|i| (i as f32 * std::f32::consts::TAU * 220.0 / RATE as f32).sin() * 0.5)
            .collect();
        let mut r = renderer();
        let emitter = emitter_at(Vec3::new(0.0, 0.0, 3.0));

        let block_a = r.render(&listener(), &[(emitter, &source)], 2400);
        let turned = Listener {
            frame: Motor3::from_rotation_translation(
                meridian_gac_core::Rotor::from_axis_angle(Vec3::Y, std::f32::consts::PI),
                Vec3::ZERO,
            ),
        };
        let block_b = r.render(&turned, &[(emitter, &source)], 2400);

        let last_a = &block_a[block_a.len() - 2..];
        let first_b = &block_b[..2];
        for ear in 0..2 {
            let step = (first_b[ear] - last_a[ear]).abs();
            assert!(
                step < 0.05,
                "ear {ear} stepped by {step} across the block boundary"
            );
        }
        // And within the second block no adjacent-sample jump exceeds
        // what a 220 Hz sine with slowly ramping parameters can produce.
        for ear in 0..2 {
            let channel: Vec<f32> = block_b.chunks_exact(2).map(|f| f[ear]).collect();
            for pair in channel.windows(2) {
                assert!(
                    (pair[1] - pair[0]).abs() < 0.08,
                    "intra-block step {}",
                    (pair[1] - pair[0]).abs()
                );
            }
        }
    }

    #[test]
    fn identical_consecutive_blocks_continue_seamlessly() {
        // Same pose, same sine phase-continued across two blocks: the
        // boundary must look like any other pair of adjacent samples.
        let sine = |offset: usize| -> Vec<f32> {
            (0..1200)
                .map(|i| {
                    ((offset + i) as f32 * std::f32::consts::TAU * 330.0 / RATE as f32).sin() * 0.4
                })
                .collect()
        };
        let mut r = renderer();
        let emitter = emitter_at(Vec3::new(2.0, 0.0, 1.0));
        let a = r.render(&listener(), &[(emitter, &sine(0))], 1200);
        let b = r.render(&listener(), &[(emitter, &sine(1200))], 1200);
        for ear in 0..2 {
            let step = (b[ear] - a[a.len() - 2 + ear]).abs();
            assert!(step < 0.03, "ear {ear} boundary step {step}");
        }
    }

    #[test]
    fn water_shrinks_the_interaural_delay() {
        // Sound crosses the head ~4.4x faster in water than in air, so
        // the ITD shrinks accordingly — the medium, not a constant,
        // decides it.
        let peak_offset = |renderer: &mut BinauralRenderer| {
            let mut source = vec![0.0f32; 256];
            source[64] = 1.0;
            // ~70 deg right: strongly lateral but neither ear is muted.
            let out = renderer.render(
                &listener(),
                &[(emitter_at(Vec3::new(1.03, 0.0, 2.82)), &source)],
                256,
            );
            let peak = |ear: usize| {
                out.chunks_exact(2)
                    .map(|f| f[ear])
                    .enumerate()
                    .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                    .map(|(i, _)| i)
                    .unwrap()
            };
            peak(0) as i64 - peak(1) as i64
        };

        let mut air = renderer();
        let mut water = renderer();
        water.medium = AcousticMedium::fresh_water();
        let air_offset = peak_offset(&mut air);
        let water_offset = peak_offset(&mut water);
        assert!(
            air_offset >= 20,
            "air ITD should be ~29 samples, got {air_offset}"
        );
        assert!(
            water_offset <= air_offset / 3,
            "water ITD {water_offset} should be far below air {air_offset}"
        );
    }
}
