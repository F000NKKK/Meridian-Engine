//! The port through which layout learns how big text is.

use crate::Size;

/// Measures text for layout. The real implementation sits on the font
/// stack (a later ADR); [`crate::FixedAdvanceText`] serves headless use
/// and tests.
pub trait TextMeasurer {
    /// The size of `text` set on one line at `font_size` pixels.
    fn measure(&self, text: &str, font_size: f32) -> Size;
}
