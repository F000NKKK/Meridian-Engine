//! An edge of the graph.

use super::{Condition, StateId};

/// A way from one state to another.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    /// The state it leaves; none leaves from any state.
    pub from: Option<StateId>,
    /// The state it enters.
    pub to: StateId,
    /// All of these must hold; none holds always.
    pub conditions: Vec<Condition>,
    /// Seconds of crossfade into the new state; zero cuts.
    pub fade: f32,
}
