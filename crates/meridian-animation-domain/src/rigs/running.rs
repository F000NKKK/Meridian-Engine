//! One mix being played: which animations, and how far in.

use meridian_gac::{Rotor, Vec3};

use super::mix::length;
use super::yaw::{wrapped, yaw_of};
use super::{AnimationLayer, Mix, Rig, RootDelta, RootMotion};
use crate::{JointTransform, Playback, Repeat};

/// What is playing now: a mix of animations and how far through one pass of
/// it the playback is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Running {
    pub(super) mix: Mix,
    pub(super) playback: Playback,
}

impl Running {
    pub(super) fn new(mix: Mix, repeat: Repeat) -> Self {
        Self {
            mix,
            playback: Playback {
                repeat,
                ..Playback::default()
            },
        }
    }

    /// How far through a pass the playback is, from zero to one.
    fn phase(&self, rig: &Rig) -> f32 {
        let duration = self.mix.duration(rig);
        if duration > 0.0 {
            self.playback.time / duration
        } else {
            0.0
        }
    }

    pub(super) fn advance(&mut self, rig: &Rig, delta: f32) {
        let duration = self.mix.duration(rig);
        self.playback.advance(delta, duration);
        let phase = self.phase(rig);
        self.mix.seek(rig, phase);
    }

    /// Swaps the shares for `mix`, keeping the place in a pass.
    pub(super) fn reweigh(&mut self, rig: &Rig, mix: Mix) {
        let phase = self.phase(rig);
        self.mix = mix;
        self.mix.seek(rig, phase);
        self.playback.time = phase * self.mix.duration(rig);
    }

    /// The layers of the mix, each weight scaled by `share`.
    pub(super) fn parts(&self, share: f32) -> impl Iterator<Item = (AnimationLayer, f32)> + '_ {
        self.mix.entries().iter().map(move |entry| {
            let layer = AnimationLayer {
                animation: entry.animation,
                time: entry.time,
                weight: 1.0,
            };
            (layer, entry.weight * share)
        })
    }

    /// Moves on as [`advance`](Self::advance) and returns how far the mix
    /// carried the joint of `pin`: the difference between where each
    /// animation puts it now and where it was, plus a whole stride for each
    /// time the pass looped, averaged by the shares.
    pub(super) fn advance_travel(&mut self, rig: &Rig, delta: f32, pin: RootMotion) -> RootDelta {
        let duration = self.mix.duration(rig);
        let before = self.phase(rig);
        let raw = if duration > 0.0 {
            before + delta * self.playback.speed / duration
        } else {
            0.0
        };
        self.advance(rig, delta);
        if duration <= 0.0 {
            return RootDelta::default();
        }
        let after = self.phase(rig);
        let strides = if self.playback.repeat == Repeat::Loop {
            raw.floor()
        } else {
            0.0
        };
        let mut total = RootDelta::default();
        for entry in self.mix.entries() {
            let length = length(rig, entry.animation);
            let span = Span {
                animation: entry.animation,
                length,
                from: before,
                to: after,
                strides,
            };
            total = total + span.travel(rig, pin) * entry.weight;
        }
        total
    }
}

/// A stretch of one animation: from phase `from` to phase `to`, with
/// `strides` whole passes besides.
struct Span {
    animation: Option<usize>,
    length: f32,
    from: f32,
    to: f32,
    strides: f32,
}

impl Span {
    fn travel(&self, rig: &Rig, pin: RootMotion) -> RootDelta {
        let at = |phase: f32| rig.joint_at(pin, self.animation, phase * self.length);
        let (start, end) = (at(self.from), at(self.to));
        let (first, last) = (at(0.0), at(1.0));
        let moved = end.translation - start.translation
            + (last.translation - first.translation) * self.strides;
        let mut travel = RootDelta {
            translation: Vec3::new(moved.x, 0.0, moved.z),
            yaw: 0.0,
        };
        if pin.follow_yaw {
            let turn = |from: JointTransform, to: JointTransform| {
                wrapped(yaw_of(to.rotation) - yaw_of(from.rotation))
            };
            travel.yaw = turn(start, end) + turn(first, last) * self.strides;
            let rest = rig.joint_at(pin, None, 0.0);
            let heading = Rotor::from_axis_angle(
                Vec3::new(0.0, 1.0, 0.0),
                yaw_of(rest.rotation) - yaw_of(start.rotation),
            );
            travel.translation = heading.transform_vector(travel.translation);
        }
        travel
    }
}
