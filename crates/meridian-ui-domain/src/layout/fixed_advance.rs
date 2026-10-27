//! A monospace-style text measurer.

use super::TextMeasurer;
use crate::Size;

/// Every character is `advance` times the font size wide and a line is
/// `line_height` times the font size tall. Deterministic, so layout tests
/// and headless tools get identical results everywhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedAdvanceText {
    pub advance: f32,
    pub line_height: f32,
}

impl Default for FixedAdvanceText {
    fn default() -> Self {
        Self {
            advance: 0.5,
            line_height: 1.0,
        }
    }
}

impl TextMeasurer for FixedAdvanceText {
    fn measure(&self, text: &str, font_size: f32) -> Size {
        let columns = text.chars().count() as f32;
        Size::new(
            columns * self.advance * font_size,
            self.line_height * font_size,
        )
    }
}
