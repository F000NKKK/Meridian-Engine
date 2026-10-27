//! [`PlayState`]: whether the simulation runs.

use substrate_data::Schema;

/// Whether the simulation runs. Variants are only ever appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Schema)]
pub enum PlayState {
    /// Stepping every frame.
    Playing,
    /// Stopped, state kept.
    Paused,
}
