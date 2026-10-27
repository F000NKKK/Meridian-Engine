//! [`SimulationControl`]: the port to whatever runs the simulation.

use meridian_editor_domain::PlayState;

/// What a [`crate::Session`] needs from the thing that steps the simulation
/// (`runtime-app`'s frame loop, a test double). The session decides when the
/// editor's `Play`, `Pause` and `Step` apply; this does the stepping.
pub trait SimulationControl: std::fmt::Debug {
    /// Whether the simulation runs by itself.
    fn state(&self) -> PlayState;

    /// Starts stepping every frame.
    fn play(&mut self);

    /// Stops stepping, keeping the state.
    fn pause(&mut self);

    /// Runs exactly one simulation step.
    fn step(&mut self);
}
