//! Frame timing: monotonic [`Clock`] producing per-frame [`Time`]
//! snapshots.

use std::time::Instant;

/// A snapshot of the engine's monotonic clock for one frame: how long the
/// previous frame took, and how long the clock has been running in total.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Time {
    pub delta_seconds: f64,
    pub total_seconds: f64,
}

/// Produces [`Time`] snapshots from `std::time::Instant`. `delta_seconds`
/// is wall-clock time since the previous [`tick`](Self::tick), not a fixed
/// step — a fixed-step accumulator for deterministic simulation is a
/// separate concern, tracked in docs/roadmap.md, not this type's job.
#[derive(Debug)]
pub struct Clock {
    start: Instant,
    last_tick: Instant,
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last_tick: now,
        }
    }

    /// Advances the clock to "now" and returns the elapsed/total time.
    pub fn tick(&mut self) -> Time {
        let now = Instant::now();
        let delta_seconds = now.duration_since(self.last_tick).as_secs_f64();
        let total_seconds = now.duration_since(self.start).as_secs_f64();
        self.last_tick = now;
        Time {
            delta_seconds,
            total_seconds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;
    #[test]
    fn clock_delta_and_total_advance_monotonically() {
        let mut clock = Clock::new();
        sleep(Duration::from_millis(10));
        let t1 = clock.tick();
        assert!(
            t1.delta_seconds >= 0.005,
            "delta should reflect the sleep, got {}",
            t1.delta_seconds
        );
        assert!(t1.total_seconds >= t1.delta_seconds);

        sleep(Duration::from_millis(10));
        let t2 = clock.tick();
        assert!(t2.delta_seconds >= 0.005);
        assert!(
            t2.total_seconds > t1.total_seconds,
            "total must keep accumulating"
        );
    }
}
