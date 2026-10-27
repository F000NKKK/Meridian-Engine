//! A grid row or column definition.

/// How a `Grid` track gets its size, as WPF's `GridLength`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridLength {
    /// Fixed pixels.
    Px(f32),
    /// As large as the largest child in the track.
    Auto,
    /// A weighted share of the space left after `Px` and `Auto` tracks
    /// (`*` is `Star(1.0)`, `2*` is `Star(2.0)`).
    Star(f32),
}
