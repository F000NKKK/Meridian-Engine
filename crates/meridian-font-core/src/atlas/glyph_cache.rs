//! Rasterize each glyph once per size and keep it on the page.

use std::collections::HashMap;

use super::cached_glyph::CachedGlyph;
use super::glyph_atlas::GlyphAtlas;
use crate::{FontError, FontFace, GlyphId, Point, rasterize};

/// Pixel sizes are remembered to a sixteenth of a pixel.
const SIZE_STEPS_PER_PIXEL: f32 = 16.0;

/// Glyphs of one face, rasterized on demand into an atlas page. When the
/// page fills up it is wiped and starts over, which bumps
/// [`version`](Self::version) so the renderer knows to re-upload it; glyphs
/// obtained before such a reset no longer point at their pixels and must be
/// requested again.
#[derive(Debug, Clone)]
pub struct GlyphCache {
    atlas: GlyphAtlas,
    glyphs: HashMap<(GlyphId, u32), CachedGlyph>,
    version: u64,
}

impl GlyphCache {
    /// A cache over an empty page of `width` by `height` pixels.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            atlas: GlyphAtlas::new(width, height),
            glyphs: HashMap::new(),
            version: 0,
        }
    }

    /// The page the cached glyphs are on.
    #[must_use]
    pub const fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
    }

    /// Changes whenever the page contents change; upload again when it does.
    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    /// `glyph` of `face` at `pixel_size` pixels per em, rasterizing it on the
    /// first request.
    ///
    /// # Errors
    ///
    /// [`FontError`] for a bad glyph id, malformed glyph data, or a glyph
    /// too large for one page.
    pub fn glyph(
        &mut self,
        face: &FontFace,
        glyph: GlyphId,
        pixel_size: f32,
    ) -> Result<CachedGlyph, FontError> {
        let key = (glyph, size_key(pixel_size));
        if let Some(&cached) = self.glyphs.get(&key) {
            return Ok(cached);
        }
        let scale = pixel_size / f32::from(face.units_per_em());
        let bitmap = rasterize(&face.outline(glyph)?, scale, Point::new(0.0, 0.0))?;
        let rect = if let Some(rect) = self.atlas.insert(&bitmap) {
            rect
        } else {
            self.reset();
            self.atlas
                .insert(&bitmap)
                .ok_or(FontError::BitmapTooLarge)?
        };
        let cached = CachedGlyph {
            rect,
            left: bitmap.left,
            top: bitmap.top,
        };
        self.glyphs.insert(key, cached);
        self.version += 1;
        Ok(cached)
    }

    fn reset(&mut self) {
        self.atlas.clear();
        self.glyphs.clear();
        self.version += 1;
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "sizes are small positive numbers; the cast saturates otherwise"
)]
fn size_key(pixel_size: f32) -> u32 {
    (pixel_size * SIZE_STEPS_PER_PIXEL).round().max(0.0) as u32
}
