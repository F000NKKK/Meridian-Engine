//! Spatial audio built on the GAC: mixer, DSP graph, listener and emitter frames.
//!
//! Listener-local convention (deliberately chosen, not inherited from
//! anywhere else — `gac-core`'s `Vec3` has no built-in "forward"): facing
//! direction is local `+X`, up is local `+Y`, and `+X × +Y = +Z` (see
//! `gac-core::Vec3::cross`), so right is local `+Z`. A listener at
//! `Motor3::identity()` therefore faces world `+X` with world `+Z` to its
//! right — this is what every azimuth/panning calculation below is
//! defined against.
//!
//! Panning is horizontal-plane-only VBAP (Vector Base Amplitude Panning,
//! simplified to 2D): for a source azimuth, find the two speakers whose
//! azimuths bracket it and distribute constant-power gain between just
//! those two — zero everywhere else. One algorithm handles every layout
//! ([`SpeakerLayout::mono`]/[`stereo_headphones`](SpeakerLayout::stereo_headphones)/
//! [`stereo_speakers`](SpeakerLayout::stereo_speakers)/[`surround_5_0`](SpeakerLayout::surround_5_0)/
//! [`surround_5_1`](SpeakerLayout::surround_5_1)), but front-only layouts
//! (stereo/headphones — no rear speaker) and full-ring layouts (5.0/5.1 —
//! real rear speakers) need different edge behavior: see
//! [`SpeakerLayout::wraps_around`] and [`fold_to_front_hemisphere`].
//!
//! No elevation/height channels (Dolby Atmos-style object audio) —
//! standard consumer speaker layouts are one horizontal plane, and
//! panning here only ever looks at horizontal azimuth. No HRTF — that
//! needs a measured impulse-response database this crate doesn't have;
//! "headphones" here means amplitude-only stereo panning across the full
//! front hemisphere, which is a real, well-known simplification (real
//! HRTF-based headphone audio can distinguish elevation and has sharper
//! front/back cues; simple amplitude panning can't, hence why front and
//! back collapse to the same centered pan — see
//! [`fold_to_front_hemisphere`]'s doc comment).

use meridian_gac_core::{Motor3, Vec3};

/// The listener's spatial frame for 3D audio.
#[derive(Debug, Clone, Copy, Default)]
pub struct Listener {
    pub frame: Motor3,
}

impl Listener {
    pub fn position(&self) -> Vec3 {
        self.frame.transform_point(Vec3::ZERO)
    }
}

/// A sound source's spatial frame + playback state.
#[derive(Debug, Clone, Copy, Default)]
pub struct Emitter {
    pub frame: Motor3,
}

impl Emitter {
    pub fn position(&self) -> Vec3 {
        self.frame.transform_point(Vec3::ZERO)
    }
}

/// A named output channel in a speaker layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    Left,
    Right,
    Center,
    LowFrequency,
    SurroundLeft,
    SurroundRight,
}

/// A speaker's horizontal position: degrees clockwise from straight ahead
/// (`0°`), matching the listener-local convention in the module doc
/// (positive = toward `+Z` = right). `None` means "never receives
/// directional panning" — real subwoofer (LFE) channels carry
/// bass-managed content derived from the other channels, not spatial
/// data; this crate doesn't implement bass management, so an LFE speaker
/// simply always gets `0.0` from [`SpeakerLayout::pan`].
#[derive(Debug, Clone, Copy)]
pub struct Speaker {
    pub channel: Channel,
    pub azimuth_degrees: Option<f32>,
}

/// A named set of output speakers and how panning should treat the space
/// between the outermost ones.
#[derive(Debug, Clone)]
pub struct SpeakerLayout {
    pub speakers: Vec<Speaker>,
    /// `true` for layouts with real rear speakers (5.0/5.1): panning
    /// wraps all the way around, so front and back are distinguishable.
    /// `false` for front-only layouts (mono/stereo/headphones): there's
    /// no rear speaker to justify blending "through the back", so a
    /// source behind the listener folds onto the equivalent front
    /// position instead — see [`fold_to_front_hemisphere`].
    pub wraps_around: bool,
}

impl SpeakerLayout {
    pub fn mono() -> Self {
        Self {
            speakers: vec![Speaker {
                channel: Channel::Center,
                azimuth_degrees: Some(0.0),
            }],
            wraps_around: false,
        }
    }

    /// Full front hemisphere (`±90°`) — a source directly to one side
    /// pans fully to that channel.
    pub fn stereo_headphones() -> Self {
        Self {
            speakers: vec![
                Speaker {
                    channel: Channel::Left,
                    azimuth_degrees: Some(-90.0),
                },
                Speaker {
                    channel: Channel::Right,
                    azimuth_degrees: Some(90.0),
                },
            ],
            wraps_around: false,
        }
    }

    /// Narrower near-field placement (`±30°`, ITU-R BS.775 stereo) — a
    /// source beyond `±30°` clamps to fully one channel rather than
    /// continuing to blend, since there's no speaker out there to blend
    /// toward.
    pub fn stereo_speakers() -> Self {
        Self {
            speakers: vec![
                Speaker {
                    channel: Channel::Left,
                    azimuth_degrees: Some(-30.0),
                },
                Speaker {
                    channel: Channel::Right,
                    azimuth_degrees: Some(30.0),
                },
            ],
            wraps_around: false,
        }
    }

    /// ITU-R BS.775 5.0: `L -30°, C 0°, R 30°, SL -110°, SR 110°`.
    pub fn surround_5_0() -> Self {
        Self {
            speakers: vec![
                Speaker {
                    channel: Channel::Left,
                    azimuth_degrees: Some(-30.0),
                },
                Speaker {
                    channel: Channel::Center,
                    azimuth_degrees: Some(0.0),
                },
                Speaker {
                    channel: Channel::Right,
                    azimuth_degrees: Some(30.0),
                },
                Speaker {
                    channel: Channel::SurroundLeft,
                    azimuth_degrees: Some(-110.0),
                },
                Speaker {
                    channel: Channel::SurroundRight,
                    azimuth_degrees: Some(110.0),
                },
            ],
            wraps_around: true,
        }
    }

    /// 5.0 plus a non-directional LFE channel.
    pub fn surround_5_1() -> Self {
        let mut layout = Self::surround_5_0();
        layout.speakers.push(Speaker {
            channel: Channel::LowFrequency,
            azimuth_degrees: None,
        });
        layout
    }

    /// Gain per speaker (in `self.speakers` order) for a source at
    /// `azimuth_degrees`. Exactly two speakers get nonzero (constant-
    /// power: their gains' squares sum to 1) unless there's only one
    /// directional speaker (gets `1.0` unconditionally — a mono layout
    /// can't pan) or none (everything `0.0` — a degenerate all-LFE
    /// layout, not a realistic case).
    pub fn pan(&self, azimuth_degrees: f32) -> Vec<(Channel, f32)> {
        let mut gains: Vec<(Channel, f32)> =
            self.speakers.iter().map(|s| (s.channel, 0.0)).collect();

        let mut directional: Vec<(usize, f32)> = self
            .speakers
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.azimuth_degrees.map(|a| (i, normalize_angle(a))))
            .collect();
        directional.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let n = directional.len();
        if n == 0 {
            return gains;
        }
        if n == 1 {
            gains[directional[0].0].1 = 1.0;
            return gains;
        }

        let mut az = normalize_angle(azimuth_degrees);
        if !self.wraps_around {
            az = fold_to_front_hemisphere(az);
        }

        if !self.wraps_around {
            let (first_idx, first_angle) = directional[0];
            let (last_idx, last_angle) = directional[n - 1];
            if az <= first_angle {
                gains[first_idx].1 = 1.0;
                return gains;
            }
            if az >= last_angle {
                gains[last_idx].1 = 1.0;
                return gains;
            }
        }

        let pair_count = if self.wraps_around { n } else { n - 1 };
        for i in 0..pair_count {
            let (idx_a, angle_a) = directional[i];
            let (idx_b, angle_b_raw) = directional[(i + 1) % n];
            let wrapping_pair = i + 1 == n;
            let angle_b = if wrapping_pair {
                angle_b_raw + 360.0
            } else {
                angle_b_raw
            };

            let mut az_adj = az;
            if wrapping_pair && az_adj < angle_a {
                az_adj += 360.0;
            }

            const EPS: f32 = 1e-4;
            if az_adj >= angle_a - EPS && az_adj <= angle_b + EPS {
                let span = angle_b - angle_a;
                let t = if span.abs() < 1e-6 {
                    0.0
                } else {
                    ((az_adj - angle_a) / span).clamp(0.0, 1.0)
                };
                let gain_a = (t * core::f32::consts::FRAC_PI_2).cos();
                let gain_b = (t * core::f32::consts::FRAC_PI_2).sin();
                gains[idx_a].1 = gain_a;
                gains[idx_b].1 = gain_b;
                return gains;
            }
        }

        // Every angle is covered by some pair when wraps_around is true
        // and the non-wrapping clamp already handled the out-of-range
        // cases above, so this is unreachable in practice — kept as a
        // safe fallback (nearest speaker, full gain) rather than a panic.
        let nearest = directional
            .iter()
            .min_by(|a, b| {
                angular_distance(az, a.1)
                    .partial_cmp(&angular_distance(az, b.1))
                    .unwrap()
            })
            .unwrap();
        gains[nearest.0].1 = 1.0;
        gains
    }
}

/// Normalizes into `(-180, 180]`.
fn normalize_angle(deg: f32) -> f32 {
    let mut a = deg % 360.0;
    if a > 180.0 {
        a -= 360.0;
    }
    if a <= -180.0 {
        a += 360.0;
    }
    a
}

fn angular_distance(a: f32, b: f32) -> f32 {
    normalize_angle(a - b).abs()
}

/// Reflects an azimuth outside `[-90°, 90°]` back into it, folding front
/// and back onto each other (`fold(180°) == 0°`, `fold(170°) == 10°`).
/// This is the physically correct behavior for amplitude-only panning
/// across a front-only speaker pair: a source directly ahead and a
/// source directly behind are equidistant from both ears/speakers, so
/// they're indistinguishable and must produce the *same* centered pan,
/// not an arbitrary left/right bias. Layouts with real rear speakers
/// ([`SpeakerLayout::wraps_around`]) skip this — they don't have the
/// ambiguity because there's an actual speaker back there to route to.
pub fn fold_to_front_hemisphere(azimuth_degrees: f32) -> f32 {
    if azimuth_degrees > 90.0 {
        180.0 - azimuth_degrees
    } else if azimuth_degrees < -90.0 {
        -180.0 - azimuth_degrees
    } else {
        azimuth_degrees
    }
}

/// Distance attenuation. OpenAL's "inverse clamped distance" model:
/// `gain == 1.0` at or inside `reference_distance`, decreasing beyond it,
/// clamped at `max_distance` so a source doesn't silently vanish to
/// exactly zero.
#[derive(Debug, Clone, Copy)]
pub struct AttenuationModel {
    pub reference_distance: f32,
    pub rolloff: f32,
    pub max_distance: f32,
}

impl Default for AttenuationModel {
    fn default() -> Self {
        Self {
            reference_distance: 1.0,
            rolloff: 1.0,
            max_distance: 1000.0,
        }
    }
}

impl AttenuationModel {
    pub fn gain(&self, distance: f32) -> f32 {
        let d = distance.clamp(self.reference_distance, self.max_distance);
        self.reference_distance
            / (self.reference_distance + self.rolloff * (d - self.reference_distance))
    }
}

/// The azimuth of `local_position` (a point already in listener-local
/// space) in the horizontal plane, per the module doc's convention:
/// `0°` = straight ahead (`+X`), positive = toward `+Z` (right).
/// Elevation (`local_position.y`) is ignored — see the module doc.
fn azimuth_of(local_position: Vec3) -> f32 {
    local_position.z.atan2(local_position.x).to_degrees()
}

/// Per-channel gain for one `emitter` as heard by `listener` through
/// `layout`, combining direction ([`SpeakerLayout::pan`]) and distance
/// (`attenuation`). This is what [`Mixer::mix`] calls per emitter — exposed
/// standalone so a single emitter/listener pair can be tested in
/// isolation without constructing a [`Mixer`].
pub fn spatial_gains(
    listener: &Listener,
    emitter: &Emitter,
    layout: &SpeakerLayout,
    attenuation: &AttenuationModel,
) -> Vec<(Channel, f32)> {
    let local = listener.frame.inverse().transform_point(emitter.position());
    let distance_gain = attenuation.gain(local.length());
    let azimuth = azimuth_of(local);
    layout
        .pan(azimuth)
        .into_iter()
        .map(|(channel, gain)| (channel, gain * distance_gain))
        .collect()
}

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
    /// `AudioStream::push_samples` (and [`AudioOutput`]) consumes.
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

/// A real audio output for a [`SpeakerLayout`]: the bridge from this
/// crate's mixed samples to `audio-driver`'s device stream (the one
/// dependency edge a `*-core` is allowed on its own `*-driver` — see
/// docs/dependency-rules.md rule 1). Owns an open
/// `audio-driver::AudioStream` with one stream channel per speaker, in
/// speaker order — exactly the interleaving [`Mixer::render_interleaved`]
/// produces.
pub struct AudioOutput {
    stream: meridian_audio_driver::AudioStream,
}

impl AudioOutput {
    /// Opens the default output device with one channel per speaker in
    /// `layout`. Async because device enumeration is genuine I/O
    /// (`audio-driver::AudioDevice::new`, ADR 009); everything after the
    /// device exists is synchronous.
    pub async fn open(
        layout: &SpeakerLayout,
        sample_rate: u32,
    ) -> Result<Self, meridian_audio_driver::AudioDeviceError> {
        let device = meridian_audio_driver::AudioDevice::new().await?;
        let stream = device.open_stream(meridian_audio_driver::AudioStreamConfig {
            sample_rate,
            channels: layout.speakers.len() as u16,
            buffer_frames: None,
        })?;
        Ok(Self { stream })
    }

    /// Pushes interleaved samples (as produced by
    /// [`Mixer::render_interleaved`]) for playback. Blocks when the
    /// stream's ring buffer is full — the intended backpressure: a loop
    /// that renders a block and pushes it runs at the hardware's real
    /// playback rate.
    pub fn push_interleaved(&self, samples: &[f32]) {
        self.stream.push_samples(samples);
    }

    /// `true` if `sample_count` more samples fit without blocking.
    pub fn can_push(&self, sample_count: usize) -> bool {
        self.stream.can_push(sample_count)
    }
}

/// Distance between the ears — the standard ~head-width used for ITD.
const EAR_SEPARATION_METERS: f32 = 0.21;
const SPEED_OF_SOUND_M_PER_S: f32 = 343.0;

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
    fn new(sample_rate: u32) -> Self {
        // Longest ITD at 48 kHz is ~30 samples; 256 leaves headroom for
        // any sample rate this crate will realistically see.
        let capacity = ((EAR_SEPARATION_METERS / SPEED_OF_SOUND_M_PER_S * sample_rate as f32)
            as usize
            + 8)
        .max(64);
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
/// - **Distance**: the same [`AttenuationModel`] as the mixer.
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
    sources: Vec<BinauralSourceState>,
}

impl BinauralRenderer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            attenuation: AttenuationModel::default(),
            sources: Vec::new(),
        }
    }

    pub fn with_attenuation(mut self, attenuation: AttenuationModel) -> Self {
        self.attenuation = attenuation;
        self
    }

    /// One-pole coefficient for a cutoff frequency at this sample rate.
    fn alpha_for_cutoff(&self, cutoff_hz: f32) -> f32 {
        1.0 - (-std::f32::consts::TAU * cutoff_hz / self.sample_rate as f32).exp()
    }

    /// Where `emitter` sits relative to `listener`, translated into
    /// per-ear gain/delay/filter targets.
    fn targets(&self, listener: &Listener, emitter: &Emitter) -> EarTargets {
        let local = listener.frame.inverse().transform_point(emitter.position());
        let distance_gain = self.attenuation.gain(local.length());

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
        let rear_gain = 1.0 - 0.25 * rear;
        let rear_cutoff_scale = 1.0 - 0.55 * rear;

        // ITD: a source to the right (+sin_az) reaches the left ear later.
        let itd_samples =
            EAR_SEPARATION_METERS / SPEED_OF_SOUND_M_PER_S * self.sample_rate as f32;
        let delay_l = (sin_az * itd_samples).max(0.0);
        let delay_r = (-sin_az * itd_samples).max(0.0);

        // Head shadow: the ear opposite the source loses highs. Cutoff
        // slides from "transparent" toward ~900 Hz as the source moves
        // fully to the other side.
        let shadow_l = sin_az.max(0.0); // source right -> left ear shadowed
        let shadow_r = (-sin_az).max(0.0);
        let cutoff = |shadow: f32| {
            let open = 18_000.0_f32.min(self.sample_rate as f32 * 0.45);
            (open + (900.0 - open) * shadow) * rear_cutoff_scale
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
            self.sources.push(BinauralSourceState::new(self.sample_rate));
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

/// A single DSP effect: mutates a buffer of samples in place.
pub trait DspNode {
    fn process(&mut self, samples: &mut [f32]);
}

/// Scales every sample by a fixed factor.
#[derive(Debug, Clone, Copy)]
pub struct Gain(pub f32);

impl DspNode for Gain {
    fn process(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() {
            *s *= self.0;
        }
    }
}

/// A one-pole IIR low-pass filter: `state += alpha * (input - state)`.
/// `alpha` close to `1.0` barely filters; close to `0.0` filters heavily.
/// Useful for the well-known "distant sounds lose high frequencies"
/// effect (air absorption) — this crate doesn't wire that up
/// automatically from [`AttenuationModel`], but the piece is here.
#[derive(Debug, Clone, Copy)]
pub struct LowPassFilter {
    pub alpha: f32,
    state: f32,
}

impl LowPassFilter {
    pub fn new(alpha: f32) -> Self {
        Self { alpha, state: 0.0 }
    }
}

impl DspNode for LowPassFilter {
    fn process(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() {
            self.state += self.alpha * (*s - self.state);
            *s = self.state;
        }
    }
}

/// A chain of DSP effect nodes, applied in order.
#[derive(Default)]
pub struct DspGraph {
    nodes: Vec<Box<dyn DspNode>>,
}

impl DspGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, node: impl DspNode + 'static) {
        self.nodes.push(Box::new(node));
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        for node in &mut self.nodes {
            node.process(samples);
        }
    }
}

#[cfg(test)]
mod render_and_output_tests {
    use super::*;

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

    #[test]
    fn render_interleaved_matches_mix_per_frame() {
        let mixer = Mixer::new(SpeakerLayout::stereo_headphones());
        let emitter = emitter_at(Vec3::new(1.0, 0.0, 2.0));
        let source = [0.25f32, -0.5, 1.0];

        let out = mixer.render_interleaved(&listener(), &[(emitter, &source)], source.len());
        assert_eq!(out.len(), source.len() * 2);

        for (frame, sample) in source.iter().enumerate() {
            let per_channel = mixer.mix(&listener(), &[(emitter, *sample)]);
            assert!((out[frame * 2] - per_channel[0].1).abs() < 1e-6);
            assert!((out[frame * 2 + 1] - per_channel[1].1).abs() < 1e-6);
        }
    }

    #[test]
    fn render_interleaved_sums_multiple_sources() {
        let mixer = Mixer::new(SpeakerLayout::mono());
        let a = emitter_at(Vec3::new(1.0, 0.0, 0.0));
        let b = emitter_at(Vec3::new(1.0, 0.0, 0.0));
        let source = [0.5f32];

        let one = mixer.render_interleaved(&listener(), &[(a, &source)], 1);
        let two = mixer.render_interleaved(&listener(), &[(a, &source), (b, &source)], 1);
        assert!((two[0] - 2.0 * one[0]).abs() < 1e-6);
    }

    #[test]
    fn render_interleaved_pads_short_sources_with_silence() {
        let mixer = Mixer::new(SpeakerLayout::stereo_headphones());
        let emitter = emitter_at(Vec3::new(1.0, 0.0, 0.0));
        let source = [1.0f32];

        let out = mixer.render_interleaved(&listener(), &[(emitter, &source)], 3);
        assert_eq!(out.len(), 6);
        assert!(out[0] > 0.0);
        assert_eq!(&out[2..], &[0.0, 0.0, 0.0, 0.0]);
    }

    #[tokio::test]
    async fn audio_output_opens_for_every_layout_or_skips() {
        for layout in [SpeakerLayout::mono(), SpeakerLayout::stereo_headphones()] {
            match AudioOutput::open(&layout, 48000).await {
                Ok(output) => {
                    let silence = vec![0.0f32; 64 * layout.speakers.len()];
                    output.push_interleaved(&silence);
                    assert!(output.can_push(1));
                }
                Err(err) => {
                    eprintln!("skipping: no audio device available ({err})");
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listener_at_origin_facing_x() -> Listener {
        Listener {
            frame: Motor3::identity(),
        }
    }

    fn emitter_at(position: Vec3) -> Emitter {
        Emitter {
            frame: Motor3::translation(position),
        }
    }

    /// A model with `gain == 1.0` for any distance used in these tests
    /// (reference_distance far beyond them) — isolates *direction* tests
    /// from *distance* attenuation, which has its own dedicated tests.
    fn no_attenuation() -> AttenuationModel {
        AttenuationModel {
            reference_distance: 1000.0,
            rolloff: 1.0,
            max_distance: 1000.0,
        }
    }

    fn gain_of(gains: &[(Channel, f32)], channel: Channel) -> f32 {
        gains
            .iter()
            .find(|(c, _)| *c == channel)
            .map(|(_, g)| *g)
            .unwrap_or(0.0)
    }

    // ---- direction, stereo (headphones: full ±90° hemisphere) ----

    #[test]
    fn stereo_headphones_front_source_is_centered() {
        let listener = listener_at_origin_facing_x();
        let emitter = emitter_at(Vec3::new(5.0, 0.0, 0.0));
        let gains = spatial_gains(
            &listener,
            &emitter,
            &SpeakerLayout::stereo_headphones(),
            &no_attenuation(),
        );
        let (l, r) = (
            gain_of(&gains, Channel::Left),
            gain_of(&gains, Channel::Right),
        );
        assert!(
            (l - r).abs() < 1e-4,
            "front source must be centered, got L={l} R={r}"
        );
        assert!(l > 0.5, "centered gain should be ~0.707, got {l}");
    }

    #[test]
    fn stereo_headphones_behind_source_is_also_centered() {
        // The well-known limitation this module doc explains: without
        // HRTF, amplitude-only stereo can't distinguish front from back —
        // both must produce the same centered pan.
        let listener = listener_at_origin_facing_x();
        let front = emitter_at(Vec3::new(5.0, 0.0, 0.0));
        let behind = emitter_at(Vec3::new(-5.0, 0.0, 0.0));
        let attenuation = AttenuationModel::default();
        let layout = SpeakerLayout::stereo_headphones();
        let front_gains = spatial_gains(&listener, &front, &layout, &attenuation);
        let behind_gains = spatial_gains(&listener, &behind, &layout, &attenuation);
        assert!(
            (gain_of(&front_gains, Channel::Left) - gain_of(&behind_gains, Channel::Left)).abs()
                < 1e-4
        );
        assert!(
            (gain_of(&front_gains, Channel::Right) - gain_of(&behind_gains, Channel::Right)).abs()
                < 1e-4
        );
    }

    #[test]
    fn stereo_headphones_left_source_is_full_left() {
        let listener = listener_at_origin_facing_x();
        let emitter = emitter_at(Vec3::new(0.0, 0.0, -5.0));
        let gains = spatial_gains(
            &listener,
            &emitter,
            &SpeakerLayout::stereo_headphones(),
            &no_attenuation(),
        );
        assert!((gain_of(&gains, Channel::Left) - 1.0).abs() < 1e-4);
        assert!(gain_of(&gains, Channel::Right) < 1e-4);
    }

    #[test]
    fn stereo_headphones_right_source_is_full_right() {
        let listener = listener_at_origin_facing_x();
        let emitter = emitter_at(Vec3::new(0.0, 0.0, 5.0));
        let gains = spatial_gains(
            &listener,
            &emitter,
            &SpeakerLayout::stereo_headphones(),
            &no_attenuation(),
        );
        assert!((gain_of(&gains, Channel::Right) - 1.0).abs() < 1e-4);
        assert!(gain_of(&gains, Channel::Left) < 1e-4);
    }

    // ---- headphones vs speakers: narrower stereo clamps sooner ----

    #[test]
    fn narrow_stereo_speakers_clamp_beyond_their_own_placement_while_headphones_still_blend() {
        // A source at -60°: inside headphones' ±90° span (blends), but
        // beyond speakers' ±30° span (clamps to full left).
        let listener = listener_at_origin_facing_x();
        // azimuth -60° at some radius: x = r*cos(-60°)... easier to place
        // directly via local-space intent: forward=x, right=z, so
        // azimuth = atan2(z, x). -60 degrees => z = -sin(60), x = cos(60).
        let angle = (-60.0_f32).to_radians();
        let emitter = emitter_at(Vec3::new(angle.cos() * 5.0, 0.0, angle.sin() * 5.0));
        let attenuation = no_attenuation();

        let headphone_gains = spatial_gains(
            &listener,
            &emitter,
            &SpeakerLayout::stereo_headphones(),
            &attenuation,
        );
        let speaker_gains = spatial_gains(
            &listener,
            &emitter,
            &SpeakerLayout::stereo_speakers(),
            &attenuation,
        );

        assert!(
            gain_of(&headphone_gains, Channel::Right) > 1e-3,
            "headphones should still blend in some right at -60deg"
        );
        assert!(
            gain_of(&speaker_gains, Channel::Right) < 1e-4,
            "narrow speakers should already be fully clamped left at -60deg"
        );
        assert!((gain_of(&speaker_gains, Channel::Left) - 1.0).abs() < 1e-4);
    }

    // ---- 5.0/5.1: real rear speakers distinguish front from back ----

    #[test]
    fn surround_5_0_front_source_goes_mostly_to_center() {
        let listener = listener_at_origin_facing_x();
        let emitter = emitter_at(Vec3::new(5.0, 0.0, 0.0));
        let gains = spatial_gains(
            &listener,
            &emitter,
            &SpeakerLayout::surround_5_0(),
            &no_attenuation(),
        );
        assert!((gain_of(&gains, Channel::Center) - 1.0).abs() < 1e-4);
        assert!(gain_of(&gains, Channel::SurroundLeft) < 1e-4);
        assert!(gain_of(&gains, Channel::SurroundRight) < 1e-4);
    }

    #[test]
    fn surround_5_0_behind_source_splits_between_surrounds_not_center() {
        // Unlike stereo, 5.0 has real rear speakers: front and back are
        // now distinguishable, and a dead-behind source must not leak
        // into the front Center channel at all.
        let listener = listener_at_origin_facing_x();
        let emitter = emitter_at(Vec3::new(-5.0, 0.0, 0.0));
        let gains = spatial_gains(
            &listener,
            &emitter,
            &SpeakerLayout::surround_5_0(),
            &no_attenuation(),
        );
        assert_eq!(
            gain_of(&gains, Channel::Center),
            0.0,
            "a dead-behind source must not leak into the front Center channel"
        );
        let (sl, sr) = (
            gain_of(&gains, Channel::SurroundLeft),
            gain_of(&gains, Channel::SurroundRight),
        );
        assert!(
            (sl - sr).abs() < 1e-4,
            "dead-behind should split evenly between the two surrounds, got SL={sl} SR={sr}"
        );
        assert!(sl > 0.5);
    }

    #[test]
    fn surround_5_0_left_source_goes_to_left_speaker() {
        let listener = listener_at_origin_facing_x();
        let emitter = emitter_at(Vec3::new(0.0, 0.0, -5.0));
        let gains = spatial_gains(
            &listener,
            &emitter,
            &SpeakerLayout::surround_5_0(),
            &no_attenuation(),
        );
        // -90 degrees falls between L(-30) and SL(-110).
        assert!(gain_of(&gains, Channel::Left) > 0.0);
        assert!(gain_of(&gains, Channel::SurroundLeft) > 0.0);
        assert_eq!(gain_of(&gains, Channel::Right), 0.0);
        assert_eq!(gain_of(&gains, Channel::Center), 0.0);
    }

    #[test]
    fn surround_5_1_lfe_never_receives_directional_gain() {
        let listener = listener_at_origin_facing_x();
        for position in [
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(-5.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, -5.0),
        ] {
            let emitter = emitter_at(position);
            let gains = spatial_gains(
                &listener,
                &emitter,
                &SpeakerLayout::surround_5_1(),
                &no_attenuation(),
            );
            assert_eq!(gain_of(&gains, Channel::LowFrequency), 0.0);
        }
    }

    #[test]
    fn mono_always_gets_full_gain_regardless_of_direction() {
        let listener = listener_at_origin_facing_x();
        for position in [
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(-5.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, -5.0),
        ] {
            let emitter = emitter_at(position);
            let gains = spatial_gains(
                &listener,
                &emitter,
                &SpeakerLayout::mono(),
                &no_attenuation(),
            );
            assert_eq!(gain_of(&gains, Channel::Center), 1.0);
        }
    }

    // ---- distance attenuation ----

    #[test]
    fn attenuation_gain_is_one_at_or_inside_reference_distance() {
        let model = AttenuationModel::default();
        assert_eq!(model.gain(0.5), 1.0);
        assert_eq!(model.gain(1.0), 1.0);
    }

    #[test]
    fn attenuation_gain_decreases_with_distance() {
        let model = AttenuationModel::default();
        assert!(model.gain(2.0) < model.gain(1.0));
        assert!(model.gain(10.0) < model.gain(2.0));
    }

    #[test]
    fn closer_source_is_louder_than_farther_source_same_direction() {
        let listener = listener_at_origin_facing_x();
        let near = emitter_at(Vec3::new(2.0, 0.0, 0.0));
        let far = emitter_at(Vec3::new(20.0, 0.0, 0.0));
        let attenuation = AttenuationModel::default();
        let layout = SpeakerLayout::mono();
        let near_gain = gain_of(
            &spatial_gains(&listener, &near, &layout, &attenuation),
            Channel::Center,
        );
        let far_gain = gain_of(
            &spatial_gains(&listener, &far, &layout, &attenuation),
            Channel::Center,
        );
        assert!(near_gain > far_gain);
    }

    // ---- mixer: multiple simultaneous sources ----

    #[test]
    fn mixer_sums_multiple_sources_linearly() {
        let listener = listener_at_origin_facing_x();
        let left_source = (emitter_at(Vec3::new(0.0, 0.0, -5.0)), 1.0);
        let right_source = (emitter_at(Vec3::new(0.0, 0.0, 5.0)), 1.0);
        let mixer = Mixer::new(SpeakerLayout::stereo_headphones());

        let mixed = mixer.mix(&listener, &[left_source, right_source]);
        let solo_left = mixer.mix(&listener, &[left_source]);
        let solo_right = mixer.mix(&listener, &[right_source]);

        assert!((gain_of(&mixed, Channel::Left) - gain_of(&solo_left, Channel::Left)).abs() < 1e-4);
        assert!(
            (gain_of(&mixed, Channel::Right) - gain_of(&solo_right, Channel::Right)).abs() < 1e-4
        );
    }

    #[test]
    fn mixer_respects_per_source_sample_amplitude() {
        let listener = listener_at_origin_facing_x();
        let mixer = Mixer::new(SpeakerLayout::mono());
        let quiet = mixer.mix(&listener, &[(emitter_at(Vec3::new(5.0, 0.0, 0.0)), 0.1)]);
        let loud = mixer.mix(&listener, &[(emitter_at(Vec3::new(5.0, 0.0, 0.0)), 0.9)]);
        assert!(gain_of(&loud, Channel::Center) > gain_of(&quiet, Channel::Center) * 5.0);
    }

    // ---- DSP graph ----

    #[test]
    fn gain_node_scales_samples() {
        let mut graph = DspGraph::new();
        graph.push(Gain(0.5));
        let mut samples = [1.0, 2.0, -4.0];
        graph.process(&mut samples);
        assert_eq!(samples, [0.5, 1.0, -2.0]);
    }

    #[test]
    fn low_pass_filter_smooths_a_step_input_toward_target() {
        let mut filter = LowPassFilter::new(0.1);
        let mut samples = [1.0; 50];
        filter.process(&mut samples);
        assert!(samples[0] < 0.5, "first sample should barely move from 0");
        assert!(
            samples[49] > 0.98,
            "after many samples the filter should have converged near 1.0"
        );
    }

    #[test]
    fn dsp_graph_chains_nodes_in_order() {
        let mut graph = DspGraph::new();
        graph.push(Gain(2.0));
        graph.push(Gain(3.0));
        let mut samples = [1.0];
        graph.process(&mut samples);
        assert_eq!(samples, [6.0]);
    }

    // ---- fold_to_front_hemisphere itself ----

    #[test]
    fn fold_maps_directly_behind_to_directly_ahead() {
        assert!((fold_to_front_hemisphere(180.0) - 0.0).abs() < 1e-4);
    }

    #[test]
    fn fold_is_identity_within_the_front_hemisphere() {
        assert_eq!(fold_to_front_hemisphere(45.0), 45.0);
        assert_eq!(fold_to_front_hemisphere(-45.0), -45.0);
    }

    #[test]
    fn fold_is_continuous_at_the_ninety_degree_boundary() {
        assert!((fold_to_front_hemisphere(90.0) - fold_to_front_hemisphere(90.001)).abs() < 0.01);
        assert!((fold_to_front_hemisphere(-90.0) - fold_to_front_hemisphere(-90.001)).abs() < 0.01);
    }
}

#[cfg(test)]
mod binaural_tests {
    use super::*;

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
        let out = r.render(
            &listener(),
            &[(emitter_at(Vec3::new(0.0, 0.0, 3.0)), &source)],
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
        let high: Vec<f32> = (0..4096).map(|i| if i % 2 == 0 { 0.5 } else { -0.5 }).collect();
        let low = vec![0.5f32; 4096];
        let position = Vec3::new(0.0, 0.0, 3.0); // hard right

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
}
