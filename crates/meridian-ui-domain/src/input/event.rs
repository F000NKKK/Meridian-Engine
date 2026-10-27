//! Input as the UI receives it.

use super::{Key, Modifiers, PointerButton};
use crate::Point;

/// One input event, with pointer positions in the UI root's coordinates
/// (viewport pixels).
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    PointerMoved(Point),
    PointerPressed {
        button: PointerButton,
        at: Point,
    },
    PointerReleased {
        button: PointerButton,
        at: Point,
    },
    KeyPressed {
        key: Key,
        modifiers: Modifiers,
    },
    /// Typed text, after the platform's layout and IME processing.
    Text(String),
}
