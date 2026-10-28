//! Where a clip is in time.

use super::Repeat;

/// The state of one clip being played: how far in, how fast, and what to do
/// at the end. It does not own the clip, so one clip can drive many
/// playbacks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Playback {
    /// Seconds into the clip.
    pub time: f32,
    /// Seconds of clip per second of real time; negative plays backwards.
    pub speed: f32,
    /// What happens at either end.
    pub repeat: Repeat,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            time: 0.0,
            speed: 1.0,
            repeat: Repeat::Loop,
        }
    }
}

impl Playback {
    /// Moves `delta` real seconds along a clip of `duration` seconds.
    pub fn advance(&mut self, delta: f32, duration: f32) {
        let next = self.time + delta * self.speed;
        self.time = if duration <= 0.0 {
            0.0
        } else {
            match self.repeat {
                Repeat::Once => next.clamp(0.0, duration),
                Repeat::Loop => next.rem_euclid(duration),
            }
        };
    }

    /// Whether a [`Repeat::Once`] playback has reached the end of a clip of
    /// `duration` seconds. A looping one never finishes.
    #[must_use]
    pub fn finished(self, duration: f32) -> bool {
        self.repeat == Repeat::Once && self.time >= duration
    }
}
