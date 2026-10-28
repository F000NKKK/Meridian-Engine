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
    /// The wheel or touchpad turned over `at`; positive `delta` scrolls
    /// towards the end of the content (down, right), in pixels.
    Scrolled {
        at: Point,
        delta: Point,
    },
    /// Typed text, after the platform's layout and IME processing.
    Text(String),
}
