//! A timed change from one thing playing to another.

/// How far a crossfade has got: the weight of the incoming animation climbs
/// from zero to one over `duration` seconds, easing in and out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossfade {
    duration: f32,
    elapsed: f32,
}

impl Crossfade {
    /// A fade of `duration` seconds; zero or less is over at once.
    #[must_use]
    pub fn new(duration: f32) -> Self {
        Self {
            duration: duration.max(0.0),
            elapsed: 0.0,
        }
    }

    /// Moves `delta` seconds on, without passing the end.
    pub fn advance(&mut self, delta: f32) {
        self.elapsed = (self.elapsed + delta).min(self.duration);
    }

    /// How much of the incoming animation shows: zero at the start, one at
    /// the end, smoothly in between.
    #[must_use]
    pub fn weight(self) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        let t = self.elapsed / self.duration;
        t * t * (3.0 - 2.0 * t)
    }

    /// Whether the incoming animation has fully taken over.
    #[must_use]
    pub fn finished(self) -> bool {
        self.elapsed >= self.duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_weight_eases_from_zero_to_one_and_stops() {
        let mut fade = Crossfade::new(2.0);
        assert!(fade.weight().abs() < f32::EPSILON);
        fade.advance(1.0);
        assert!((fade.weight() - 0.5).abs() < 1e-6);
        assert!(!fade.finished());
        fade.advance(5.0);
        assert!((fade.weight() - 1.0).abs() < f32::EPSILON);
        assert!(fade.finished());
    }

    #[test]
    fn an_instant_fade_is_already_over() {
        let fade = Crossfade::new(0.0);
        assert!(fade.finished());
        assert!((fade.weight() - 1.0).abs() < f32::EPSILON);
        assert!(Crossfade::new(-1.0).finished());
    }
}
