//! Playing a rig's animations, one at a time or crossfading (ADR 047).

use meridian_gac::Vec3;

use super::{Animation, AnimationLayer, Rig, RootMotion};
use crate::{Crossfade, Playback, Repeat};

/// One animation being played: which, and how far.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Running {
    animation: Option<usize>,
    playback: Playback,
}

impl Running {
    fn advance(&mut self, rig: &Rig, delta: f32) {
        let duration = self
            .animation
            .and_then(|index| rig.animations.get(index))
            .map_or(0.0, Animation::duration);
        self.playback.advance(delta, duration);
    }

    /// Moves on as [`advance`](Self::advance) and returns how far the
    /// animation carried the joint of `pin` along the ground: the difference
    /// between where it is now and where it was, plus a whole stride for each
    /// time the animation looped.
    fn advance_travel(&mut self, rig: &Rig, delta: f32, pin: RootMotion) -> Vec3 {
        let duration = self
            .animation
            .and_then(|index| rig.animations.get(index))
            .map_or(0.0, Animation::duration);
        let before = self.playback.time;
        let raw = before + delta * self.playback.speed;
        self.advance(rig, delta);
        if duration <= 0.0 {
            return Vec3::ZERO;
        }
        let at = |time: f32| rig.joint_at(pin, self.animation, time).translation;
        let mut travel = at(self.playback.time) - at(before);
        if self.playback.repeat == Repeat::Loop {
            travel = travel + (at(duration) - at(0.0)) * (raw / duration).floor();
        }
        Vec3::new(travel.x, 0.0, travel.z)
    }

    fn layer(self, weight: f32) -> AnimationLayer {
        AnimationLayer {
            animation: self.animation,
            time: self.playback.time,
            weight,
        }
    }
}

/// The state of what a model is playing: an animation and, while one is
/// being replaced, the one leaving. Holds no rig, so it can sit in a
/// component next to a handle to one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationPlayer {
    current: Running,
    leaving: Option<(Running, Crossfade)>,
}

impl AnimationPlayer {
    /// A player on `animation` (none is the rest pose), looping from the start.
    #[must_use]
    pub fn new(animation: Option<usize>) -> Self {
        Self {
            current: Running {
                animation,
                playback: Playback::default(),
            },
            leaving: None,
        }
    }

    /// The animation playing, or arriving while a crossfade runs.
    #[must_use]
    pub fn animation(&self) -> Option<usize> {
        self.current.animation
    }

    /// Whether a crossfade is still running.
    #[must_use]
    pub fn is_fading(&self) -> bool {
        self.leaving.is_some()
    }

    /// Whether the animation playing has reached its end: only a
    /// [`Repeat::Once`] one does.
    #[must_use]
    pub fn finished(&self, rig: &Rig) -> bool {
        let duration = self
            .current
            .animation
            .and_then(|index| rig.animations.get(index))
            .map_or(0.0, Animation::duration);
        self.current.playback.finished(duration)
    }

    /// Switches to `animation` at once, from its start.
    pub fn play(&mut self, animation: Option<usize>, repeat: Repeat) {
        self.current = Running {
            animation,
            playback: Playback {
                repeat,
                ..Playback::default()
            },
        };
        self.leaving = None;
    }

    /// Fades to `animation` over `seconds`, starting it from its beginning
    /// while the one playing carries on underneath. Fading again before the
    /// last fade ends drops the animation that was still leaving.
    pub fn crossfade_to(&mut self, animation: Option<usize>, seconds: f32, repeat: Repeat) {
        let leaving = self.current;
        self.play(animation, repeat);
        self.leaving = Some((leaving, Crossfade::new(seconds)));
    }

    /// Moves `delta` seconds on, ending a crossfade that has finished.
    pub fn advance(&mut self, rig: &Rig, delta: f32) {
        self.current.advance(rig, delta);
        if let Some((leaving, fade)) = &mut self.leaving {
            leaving.advance(rig, delta);
            fade.advance(delta);
            if fade.finished() {
                self.leaving = None;
            }
        }
    }

    /// Moves on as [`advance`](Self::advance) and returns how far the
    /// joint of `pin` travelled along the ground, in the model's space,
    /// mixed by the same weights as the layers (the leaving animation
    /// counting for what is left of the fade). Hand this to the entity and
    /// pose the model with [`Rig::matrices_pinned`].
    pub fn advance_travel(&mut self, rig: &Rig, delta: f32, pin: RootMotion) -> Vec3 {
        let weight = self.leaving.map_or(1.0, |(_, fade)| fade.weight());
        let mut travel = self.current.advance_travel(rig, delta, pin) * weight;
        if let Some((leaving, fade)) = &mut self.leaving {
            travel = travel + leaving.advance_travel(rig, delta, pin) * (1.0 - weight);
            fade.advance(delta);
            if fade.finished() {
                self.leaving = None;
            }
        }
        travel
    }

    /// The layers to mix for [`Rig::matrices_mixed`] and
    /// [`Rig::weights_mixed`]: the leaving animation, if any, then the
    /// current one at the fade's weight.
    #[must_use]
    pub fn layers(&self) -> Vec<AnimationLayer> {
        match self.leaving {
            Some((leaving, fade)) => vec![leaving.layer(1.0), self.current.layer(fade.weight())],
            None => vec![self.current.layer(1.0)],
        }
    }
}
