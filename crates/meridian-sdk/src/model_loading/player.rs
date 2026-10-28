//! Playing a rig's animations, one at a time or crossfading (ADR 047).

use meridian_animation_domain::{Crossfade, Playback, Repeat};

use super::mixing::AnimationLayer;
use super::rig::{ModelAnimation, ModelRig};

/// One animation being played: which, and how far.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Running {
    animation: Option<usize>,
    playback: Playback,
}

impl Running {
    fn advance(&mut self, rig: &ModelRig, delta: f32) {
        let duration = self
            .animation
            .and_then(|index| rig.animations.get(index))
            .map_or(0.0, ModelAnimation::duration);
        self.playback.advance(delta, duration);
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
    pub fn advance(&mut self, rig: &ModelRig, delta: f32) {
        self.current.advance(rig, delta);
        if let Some((leaving, fade)) = &mut self.leaving {
            leaving.advance(rig, delta);
            fade.advance(delta);
            if fade.finished() {
                self.leaving = None;
            }
        }
    }

    /// The layers to mix for [`ModelRig::palettes_mixed`] and
    /// [`ModelRig::weights_mixed`]: the leaving animation, if any, then the
    /// current one at the fade's weight.
    #[must_use]
    pub fn layers(&self) -> Vec<AnimationLayer> {
        match self.leaving {
            Some((leaving, fade)) => vec![leaving.layer(1.0), self.current.layer(fade.weight())],
            None => vec![self.current.layer(1.0)],
        }
    }
}

#[cfg(test)]
mod tests {
    use meridian_animation_domain::{Clip, Interpolation, Keys};

    use super::*;
    use crate::model_loading::rig::{ModelAnimation, SkeletonClip};

    fn rig_with(durations: &[f32]) -> ModelRig {
        let clip = |duration: f32| {
            let keys = Keys::<3>::new(
                vec![0.0, duration],
                vec![[0.0; 3]; 2],
                Interpolation::Linear,
            )
            .expect("valid keys");
            Clip::new(vec![meridian_animation_domain::Channel {
                target: meridian_animation_domain::JointId::from_index(0).expect("joint 0"),
                track: meridian_animation_domain::Track::Translation(keys),
            }])
        };
        ModelRig {
            animations: durations
                .iter()
                .map(|&duration| ModelAnimation {
                    clips: vec![SkeletonClip {
                        skeleton: 0,
                        clip: clip(duration),
                    }],
                    weights: Vec::new(),
                })
                .collect(),
            ..ModelRig::default()
        }
    }

    #[test]
    fn a_player_loops_its_animation() {
        let rig = rig_with(&[2.0]);
        let mut player = AnimationPlayer::new(Some(0));
        player.advance(&rig, 3.0);
        assert_eq!(player.layers().len(), 1);
        assert!((player.layers()[0].time - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_crossfade_keeps_the_old_animation_running_until_it_ends() {
        let rig = rig_with(&[4.0, 4.0]);
        let mut player = AnimationPlayer::new(Some(0));
        player.advance(&rig, 1.0);
        player.crossfade_to(Some(1), 2.0, Repeat::Loop);
        assert!(player.is_fading());
        player.advance(&rig, 1.0);
        let layers = player.layers();
        assert_eq!(layers.len(), 2);
        assert_eq!((layers[0].animation, layers[0].time), (Some(0), 2.0));
        assert_eq!((layers[1].animation, layers[1].time), (Some(1), 1.0));
        assert!((layers[1].weight - 0.5).abs() < 1e-6);
        player.advance(&rig, 1.0);
        assert!(!player.is_fading());
        assert_eq!(player.layers().len(), 1);
        assert_eq!(player.animation(), Some(1));
    }

    #[test]
    fn play_replaces_everything_at_once() {
        let rig = rig_with(&[4.0, 4.0]);
        let mut player = AnimationPlayer::new(Some(0));
        player.crossfade_to(Some(1), 5.0, Repeat::Loop);
        player.play(None, Repeat::Once);
        player.advance(&rig, 1.0);
        assert!(!player.is_fading());
        assert_eq!(player.layers()[0].animation, None);
    }
}
