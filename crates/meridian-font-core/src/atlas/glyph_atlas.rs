//! A single-channel texture page filled by shelf packing.

use super::atlas_rect::AtlasRect;
use super::shelf::Shelf;
use crate::Bitmap;

/// Empty pixels kept around every bitmap so filtered sampling never
/// bleeds one glyph into its neighbour.
const PADDING: u32 = 1;

/// One page of 8-bit coverage, shelf-packed. It only produces bytes:
/// uploading them to a texture is the renderer's job.
#[derive(Debug, Clone)]
pub struct GlyphAtlas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    shelves: Vec<Shelf>,
}

impl GlyphAtlas {
    /// An empty page of `width` by `height` pixels.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let size = usize::try_from(u64::from(width) * u64::from(height)).unwrap_or(0);
        Self {
            width,
            height,
            pixels: vec![0; size],
            shelves: Vec::new(),
        }
    }

    /// Page width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Page height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The page, `width * height` coverage bytes, rows top to bottom.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Forgets every bitmap and blanks the page.
    pub fn clear(&mut self) {
        self.pixels.fill(0);
        self.shelves.clear();
    }

    /// Copies `bitmap` onto the page. `None` when it does not fit; an
    /// empty bitmap always fits and takes no room.
    pub fn insert(&mut self, bitmap: &Bitmap) -> Option<AtlasRect> {
        if bitmap.width == 0 || bitmap.height == 0 {
            return Some(AtlasRect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            });
        }
        let (x, y) = self.reserve(bitmap.width + PADDING, bitmap.height + PADDING)?;
        self.blit(bitmap, x, y);
        Some(AtlasRect {
            x,
            y,
            width: bitmap.width,
            height: bitmap.height,
        })
    }

    /// Finds room for a `width` by `height` block and claims it.
    fn reserve(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        if width > self.width {
            return None;
        }
        let page_width = self.width;
        let fitting = self
            .shelves
            .iter_mut()
            .filter(|shelf| shelf.height >= height && shelf.next_x + width <= page_width)
            .min_by_key(|shelf| shelf.height);
        if let Some(shelf) = fitting {
            let x = shelf.next_x;
            shelf.next_x += width;
            return Some((x, shelf.y));
        }
        let y = self
            .shelves
            .last()
            .map_or(0, |shelf| shelf.y + shelf.height);
        if y + height > self.height {
            return None;
        }
        self.shelves.push(Shelf {
            y,
            height,
            next_x: width,
        });
        Some((0, y))
    }

    fn blit(&mut self, bitmap: &Bitmap, x: u32, y: u32) {
        let width = bitmap.width as usize;
        for (row, source) in bitmap.coverage.chunks_exact(width).enumerate() {
            let start = (y as usize + row) * self.width as usize + x as usize;
            self.pixels[start..start + width].copy_from_slice(source);
        }
    }
}
