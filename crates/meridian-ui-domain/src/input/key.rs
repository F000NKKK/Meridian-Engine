//! Keys the UI itself reacts to.

/// A non-text key. Characters arrive as [`crate::InputEvent::Text`], so
/// layout-dependent printing keys are not listed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Key {
    Tab,
    Enter,
    Space,
    Escape,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
}
