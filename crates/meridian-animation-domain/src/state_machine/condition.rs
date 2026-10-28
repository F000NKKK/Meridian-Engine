//! What must hold for a transition to fire.

use super::ParamId;

/// One test of a [`Transition`](super::Transition); a transition fires when
/// all of its conditions hold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Condition {
    /// A float parameter is greater than the number.
    Above(ParamId, f32),
    /// A float parameter is less than the number.
    Below(ParamId, f32),
    /// A bool parameter has the value.
    Is(ParamId, bool),
    /// A trigger parameter was set since the last update.
    Triggered(ParamId),
    /// The state's animation, played once, has reached its end.
    Finished,
}

impl Condition {
    /// The parameter the condition reads, if it reads one.
    pub(super) fn param(self) -> Option<(ParamId, super::ParamKind)> {
        match self {
            Self::Above(id, _) | Self::Below(id, _) => Some((id, super::ParamKind::Float)),
            Self::Is(id, _) => Some((id, super::ParamKind::Bool)),
            Self::Triggered(id) => Some((id, super::ParamKind::Trigger)),
            Self::Finished => None,
        }
    }
}
