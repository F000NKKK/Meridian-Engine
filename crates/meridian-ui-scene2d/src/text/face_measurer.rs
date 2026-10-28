//! Layout's view of a font.

use meridian_text_domain::{FontFace, shape};
use meridian_ui_domain::{Size, TextMeasurer};

/// Measures text with a real face, so layout and drawing agree on sizes.
#[derive(Debug, Clone, Copy)]
pub struct FaceMeasurer<'a> {
    face: &'a FontFace,
}

impl<'a> FaceMeasurer<'a> {
    /// A measurer over `face`.
    #[must_use]
    pub const fn new(face: &'a FontFace) -> Self {
        Self { face }
    }
}

impl TextMeasurer for FaceMeasurer<'_> {
    fn measure(&self, text: &str, font_size: f32) -> Size {
        let run = shape(self.face, text, font_size);
        Size::new(run.width, run.line_height)
    }
}
