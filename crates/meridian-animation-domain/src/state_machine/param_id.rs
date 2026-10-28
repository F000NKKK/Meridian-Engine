//! The handle of a graph parameter.

/// A parameter of an [`AnimationGraph`](super::AnimationGraph), returned when
/// it is added and used to set it and to test it in a condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamId(pub(super) usize);
