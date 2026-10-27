//! [`ManualSimulation`]: a simulation that only counts its steps.

use meridian_editor_domain::PlayState;

use crate::SimulationControl;

/// A [`SimulationControl`] with nothing behind it: it remembers its state
/// and how many steps were asked for. For tests and for a headless host
/// that has no frame loop yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManualSimulation {
    state: PlayState,
    steps: u64,
}

impl ManualSimulation {
    /// A paused simulation that has not stepped.
    #[must_use]
    pub const fn new() -> Self {
        ManualSimulation {
            state: PlayState::Paused,
            steps: 0,
        }
    }

    /// How many steps were asked for (by `Step`).
    #[must_use]
    pub const fn steps(self) -> u64 {
        self.steps
    }
}

impl Default for ManualSimulation {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulationControl for ManualSimulation {
    fn state(&self) -> PlayState {
        self.state
    }

    fn play(&mut self) {
        self.state = PlayState::Playing;
    }

    fn pause(&mut self) {
        self.state = PlayState::Paused;
    }

    fn step(&mut self) {
        self.steps += 1;
    }
}
