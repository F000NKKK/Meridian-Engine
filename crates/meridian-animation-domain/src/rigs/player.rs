//! Playing a rig's animations, one at a time, blended or crossfading (ADR 047).

use super::running::Running;
use super::{AnimationLayer, Mix, Rig, RootDelta, RootMotion};
use crate::{Crossfade, Repeat};

/// The state of what a model is playing: a mix of animations and, while one
/// is being replaced, the one leaving. Holds no rig, so it can sit in a
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
            current: Running::new(Mix::single(animation), Repeat::Loop),
            leaving: None,
        }
    }

    /// The animation playing, or arriving while a crossfade runs; for a
    /// blend, the one with the largest share.
    #[must_use]
    pub fn animation(&self) -> Option<usize> {
        self.current.mix.dominant()
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
        let duration = self.current.mix.duration(rig);
        self.current.playback.finished(duration)
    }

    /// Switches to `animation` at once, from its start.
    pub fn play(&mut self, animation: Option<usize>, repeat: Repeat) {
        self.play_mix(Mix::single(animation), repeat);
    }

    /// Switches to `mix` at once, from its start.
    pub fn play_mix(&mut self, mix: Mix, repeat: Repeat) {
        self.current = Running::new(mix, repeat);
        self.leaving = None;
    }

    /// Fades to `animation` over `seconds`, starting it from its beginning
    /// while the one playing carries on underneath. Fading again before the
    /// last fade ends drops the animation that was still leaving.
    pub fn crossfade_to(&mut self, animation: Option<usize>, seconds: f32, repeat: Repeat) {
        self.crossfade_to_mix(Mix::single(animation), seconds, repeat);
    }

    /// Fades to `mix` as [`crossfade_to`](Self::crossfade_to) does.
    pub fn crossfade_to_mix(&mut self, mix: Mix, seconds: f32, repeat: Repeat) {
        let leaving = self.current;
        self.play_mix(mix, repeat);
        self.leaving = Some((leaving, Crossfade::new(seconds)));
    }

    /// Changes the shares of the mix playing, keeping its place: a blend tree
    /// calls this each frame as its parameters move.
    pub fn reweigh(&mut self, rig: &Rig, mix: Mix) {
        self.current.reweigh(rig, mix);
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
    /// joint of `pin` travelled along the ground (and turned, if `pin`
    /// follows yaw), in the model's own frame, mixed by the same weights as
    /// the layers (the leaving animation counting for what is left of the
    /// fade). Hand this to the entity and pose the model with
    /// [`Rig::matrices_pinned`].
    pub fn advance_travel(&mut self, rig: &Rig, delta: f32, pin: RootMotion) -> RootDelta {
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
    /// [`Rig::weights_mixed`]: the animations of the leaving mix, if any, then
    /// those of the current one, each carrying its share of the fade. Each
    /// layer's weight is its share of everything mixed so far, so the result
    /// is the average of all of them by their shares.
    #[must_use]
    pub fn layers(&self) -> Vec<AnimationLayer> {
        let fade = self.leaving.map_or(1.0, |(_, fade)| fade.weight());
        let mut parts: Vec<(AnimationLayer, f32)> = Vec::new();
        if let Some((leaving, _)) = &self.leaving {
            parts.extend(leaving.parts(1.0 - fade));
        }
        parts.extend(self.current.parts(fade));
        let mut total = 0.0;
        parts
            .into_iter()
            .map(|(mut layer, share)| {
                total += share;
                layer.weight = if total > 0.0 { share / total } else { 0.0 };
                layer
            })
            .collect()
    }
}
