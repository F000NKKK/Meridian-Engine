//! Whether an element takes part in layout and hit testing.

/// As WPF's `Visibility`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Visible,
    /// Takes its space but is not hit-testable.
    Hidden,
    /// Takes no space and is not hit-testable.
    Collapsed,
}
