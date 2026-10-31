//! Speaker layouts and constant-power panning between the bracketing speakers.

use crate::{Channel, Speaker};

/// A named set of output speakers and how panning should treat the space
/// between the outermost ones.
#[derive(Debug, Clone)]
pub struct SpeakerLayout {
    /// Speakers in output-channel order.
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
    /// Returns a single center-channel layout.
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
        let mut gains = vec![0.0; self.speakers.len()];
        self.pan_into(azimuth_degrees, &mut gains);
        self.speakers
            .iter()
            .zip(gains)
            .map(|(speaker, gain)| (speaker.channel, gain))
            .collect()
    }

    /// [`pan`](Self::pan) without allocating: writes the gain of
    /// `self.speakers[i]` into `gains[i]` (every slot is overwritten; a
    /// slice shorter than the layout receives only the leading speakers). A
    /// non-finite azimuth yields silence rather than NaN gains.
    pub fn pan_into(&self, azimuth_degrees: f32, gains: &mut [f32]) {
        gains.fill(0.0);
        if !azimuth_degrees.is_finite() {
            return;
        }
        let Some(extent) = self.directional_extent() else {
            return;
        };
        let mut set = |index: usize, gain: f32| {
            if let Some(slot) = gains.get_mut(index) {
                *slot = gain;
            }
        };
        if extent.count == 1 {
            set(extent.first.0, 1.0);
            return;
        }

        let mut az = normalize_angle(azimuth_degrees);
        if !self.wraps_around {
            az = fold_to_front_hemisphere(az);
            if az <= extent.first.1 {
                set(extent.first.0, 1.0);
                return;
            }
            if az >= extent.last.1 {
                set(extent.last.0, 1.0);
                return;
            }
        }

        let (lower, upper) = self.bracketing_speakers(&extent, az);
        let wrapping_pair = upper.1 <= lower.1;
        match pair_gains(lower, upper, wrapping_pair, az) {
            Some([(a, gain_a), (b, gain_b)]) => {
                set(a, gain_a);
                set(b, gain_b);
            }
            // Unreachable in practice (every angle is bracketed); a safe
            // fallback (nearest speaker, full gain) rather than a panic.
            None => set(self.nearest_directional(az).unwrap_or(extent.first.0), 1.0),
        }
    }

    /// First, last and number of the non-LFE speakers by normalized
    /// azimuth, or `None` when no speaker is directional.
    fn directional_extent(&self) -> Option<DirectionalExtent> {
        let mut extent: Option<DirectionalExtent> = None;
        for (index, angle) in self.directional() {
            extent = Some(match extent {
                None => DirectionalExtent {
                    first: (index, angle),
                    last: (index, angle),
                    count: 1,
                },
                Some(e) => DirectionalExtent {
                    first: if angle < e.first.1 {
                        (index, angle)
                    } else {
                        e.first
                    },
                    last: if angle >= e.last.1 {
                        (index, angle)
                    } else {
                        e.last
                    },
                    count: e.count + 1,
                },
            });
        }
        extent
    }

    /// `(speaker index, normalized azimuth)` of every non-LFE speaker.
    fn directional(&self) -> impl Iterator<Item = (usize, f32)> + '_ {
        self.speakers
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.azimuth_degrees.map(|a| (i, normalize_angle(a))))
    }

    /// The adjacent directional speakers around `az`: the nearest at or
    /// below it and the nearest above it, wrapping through the ring ends.
    fn bracketing_speakers(
        &self,
        extent: &DirectionalExtent,
        az: f32,
    ) -> ((usize, f32), (usize, f32)) {
        let mut lower: Option<(usize, f32)> = None;
        let mut upper: Option<(usize, f32)> = None;
        for (index, angle) in self.directional() {
            if angle <= az && lower.is_none_or(|l| angle >= l.1) {
                lower = Some((index, angle));
            }
            if angle > az && upper.is_none_or(|u| angle < u.1) {
                upper = Some((index, angle));
            }
        }
        (lower.unwrap_or(extent.last), upper.unwrap_or(extent.first))
    }

    fn nearest_directional(&self, az: f32) -> Option<usize> {
        self.directional()
            .min_by(|a, b| angular_distance(az, a.1).total_cmp(&angular_distance(az, b.1)))
            .map(|(index, _)| index)
    }
}

/// The ends of a layout's directional speakers, by normalized azimuth.
struct DirectionalExtent {
    first: (usize, f32),
    last: (usize, f32),
    count: usize,
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
