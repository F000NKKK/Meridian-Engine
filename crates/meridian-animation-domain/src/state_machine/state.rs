//! A node of the graph.

use crate::Repeat;

/// What a state plays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationState {
    /// The animation (an index into the rig's animations); none is the rest pose.
    pub animation: Option<usize>,
    /// Whether it loops or plays once and holds its last frame.
    pub repeat: Repeat,
}
