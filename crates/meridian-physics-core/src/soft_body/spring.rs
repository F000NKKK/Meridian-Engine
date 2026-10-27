//! A structural spring connecting two soft-body particles.

use meridian_gac_core::GaFlavor;

/// A structural or volume-preserving connection between two particles in
/// a [`SoftBody`](super::mass_spring::SoftBody), by index into its `positions`/`velocities`/
/// `inverse_masses`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring<F: GaFlavor> {
    pub a: usize,
    pub b: usize,
    pub rest_length: F::Scalar,
    /// Hooke's-law spring constant: force magnitude per unit of
    /// (`current_length - rest_length`) stretch/compression.
    pub stiffness: F::Scalar,
    /// Velocity-relative damping coefficient — without this, a spring
    /// oscillates indefinitely (no energy loss); with it, stretch/
    /// compression along the spring's own axis bleeds off over time,
    /// which is what actually makes a squashed ball recover to a
    /// resting shape instead of jiggling forever.
    pub damping: F::Scalar,
}
