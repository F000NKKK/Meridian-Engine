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
//! panning here only ever looks at horizontal azimuth. No measured HRTF —
//! that needs an impulse-response database this crate doesn't have. The
//! [`Mixer`] path is amplitude-only panning (front/back collapse on
//! headphones — see [`fold_to_front_hemisphere`]); the
//! [`effects::BinauralRenderer`] path is the physically-motivated
//! headphone upgrade: interaural time difference, head-shadow filtering
//! and rear damping, so front and back genuinely differ there. Effects
//! live one-per-file under [`effects`].

use meridian_gac_core::{Motor3, Vec3};

pub mod effects;

pub use effects::{BinauralRenderer, DspGraph, DspNode, Gain, LowPassFilter};

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

