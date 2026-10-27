//! Where a bitmap sits on the page.

/// A rectangle of the atlas page, in pixels from its top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasRect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl AtlasRect {
    /// The rectangle as `[x, y, width, height]` normalized to a page of
    /// `page_width` by `page_height` pixels, origin top-left: the form a
    /// sprite's `source_rect` takes.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "atlas pages are far below 2^24 pixels on a side"
    )]
    pub fn normalized(self, page_width: u32, page_height: u32) -> [f32; 4] {
        let (w, h) = (page_width as f32, page_height as f32);
        [
            self.x as f32 / w,
            self.y as f32 / h,
            self.width as f32 / w,
            self.height as f32 / h,
        ]
    }
}
