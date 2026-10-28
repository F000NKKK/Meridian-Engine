//! The handle of a graph state.

/// A state of an [`AnimationGraph`](super::AnimationGraph), returned when it
/// is added and used to name the ends of a transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateId(pub(super) usize);
