//! What the pointer is doing.

use crate::{NodeId, PointerButton};

/// The element under the pointer and the press in progress.
#[derive(Debug, Default)]
pub(super) struct PointerState {
    pub(super) hovered: Option<NodeId>,
    pub(super) pressed: Option<(PointerButton, NodeId)>,
}
