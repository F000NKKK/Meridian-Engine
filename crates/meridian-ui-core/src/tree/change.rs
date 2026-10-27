//! The property change journal.

use crate::{NodeId, PropertyId};

/// Who wrote a property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeSource {
    /// Application code or a binding.
    Code,
    /// Direct user interaction (typing, dragging); the source of
    /// `TwoWay` binding write-back.
    User,
}

/// One recorded property change, in write order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    pub node: NodeId,
    pub property: PropertyId,
    pub source: ChangeSource,
}
