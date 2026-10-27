//! Speaker layouts and constant-power panning between the bracketing speakers.

use crate::Channel;
use crate::Speaker;

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
        let directional = self.directional_speakers();
        match directional.len() {
            0 => return gains,
            1 => {
                gains[directional[0].0].1 = 1.0;
                return gains;
            }
            _ => {}
        }

        let mut az = normalize_angle(azimuth_degrees);
        if !self.wraps_around {
            az = fold_to_front_hemisphere(az);
            if let Some(end) = clamped_end(&directional, az) {
                gains[end].1 = 1.0;
                return gains;
            }
        }

        if let Some(pair) = self.bracketing_gains(&directional, az) {
            for (index, gain) in pair {
                gains[index].1 = gain;
            }
            return gains;
        }

        // Every angle is covered by some pair when wraps_around is true
        // and the non-wrapping clamp already handled the out-of-range
        // cases above, so this is unreachable in practice — kept as a
        // safe fallback (nearest speaker, full gain) rather than a panic.
        // `directional` is non-empty: the `0` arm above returned.
        if let Some(nearest) = directional
            .iter()
            .min_by(|a, b| angular_distance(az, a.1).total_cmp(&angular_distance(az, b.1)))
        {
            gains[nearest.0].1 = 1.0;
        }
        gains
    }

    /// `(speaker index, normalized azimuth)` of every non-LFE speaker,
    /// ordered by azimuth.
    fn directional_speakers(&self) -> Vec<(usize, f32)> {
        let mut directional: Vec<(usize, f32)> = self
            .speakers
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.azimuth_degrees.map(|a| (i, normalize_angle(a))))
            .collect();
        directional.sort_by(|a, b| a.1.total_cmp(&b.1));
        directional
    }

    /// The gains of the adjacent speaker pair that brackets `az`.
    fn bracketing_gains(&self, directional: &[(usize, f32)], az: f32) -> Option<[(usize, f32); 2]> {
        let n = directional.len();
        let pair_count = if self.wraps_around { n } else { n - 1 };
        (0..pair_count)
            .find_map(|i| pair_gains(directional[i], directional[(i + 1) % n], i + 1 == n, az))
    }
}

/// The speaker an azimuth lies beyond, in a layout that does not wrap
/// around (the nearest end takes full gain).
fn clamped_end(directional: &[(usize, f32)], az: f32) -> Option<usize> {
    let (first_idx, first_angle) = directional[0];
    let (last_idx, last_angle) = directional[directional.len() - 1];
    if az <= first_angle {
        Some(first_idx)
    } else if az >= last_angle {
        Some(last_idx)
    } else {
        None
    }
}

/// Constant-power gains of `a` and `b` when `az` lies between them.
fn pair_gains(
    (idx_a, angle_a): (usize, f32),
    (idx_b, angle_b_raw): (usize, f32),
    wrapping_pair: bool,
    az: f32,
) -> Option<[(usize, f32); 2]> {
    const EPS: f32 = 1e-4;
    let angle_b = if wrapping_pair {
        angle_b_raw + 360.0
    } else {
        angle_b_raw
    };
    let mut az_adj = az;
    if wrapping_pair && az_adj < angle_a {
        az_adj += 360.0;
    }
    if az_adj < angle_a - EPS || az_adj > angle_b + EPS {
        return None;
    }
    let span = angle_b - angle_a;
    let t = if span.abs() < 1e-6 {
        0.0
    } else {
        ((az_adj - angle_a) / span).clamp(0.0, 1.0)
    };
    let quarter_turn = t * core::f32::consts::FRAC_PI_2;
    Some([(idx_a, quarter_turn.cos()), (idx_b, quarter_turn.sin())])
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
