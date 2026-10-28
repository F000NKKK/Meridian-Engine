//! A node of the graph.

use super::Motion;
use crate::Repeat;

/// What a state plays.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationState {
    /// The animation, or the blend of them.
    pub motion: Motion,
    /// Whether it loops or plays once and holds its last frame.
    pub repeat: Repeat,
}
