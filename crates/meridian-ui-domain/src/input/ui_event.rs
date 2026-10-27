//! Events the UI raises for code-behind to handle.

use super::{Key, Modifiers, PointerButton};
use crate::{NodeId, Point};

/// What happened to a node.
#[derive(Debug, Clone, PartialEq)]
pub enum UiEventKind {
    PointerPressed {
        button: PointerButton,
        at: Point,
    },
    PointerReleased {
        button: PointerButton,
        at: Point,
    },
    /// Pressed and released on the same element.
    Click {
        button: PointerButton,
    },
    PointerEntered,
    PointerLeft,
    KeyPressed {
        key: Key,
        modifiers: Modifiers,
    },
    Text(String),
    FocusGained,
    FocusLost,
}

/// An event and the node it targets. Events bubble: a handler for an
/// ancestor (see [`crate::UiTree::ancestors`]) sees what happened to its
/// descendants. Enter, leave and focus events do not bubble.
#[derive(Debug, Clone, PartialEq)]
pub struct UiEvent {
    pub target: NodeId,
    pub kind: UiEventKind,
}
