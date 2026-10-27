//! Our own font stack, CPU only (see ADR 023): a TrueType face (metrics,
//! character map, kerning, glyph outlines), an area-exact anti-aliasing
//! rasterizer, a shelf-packed glyph atlas with a cache, and a simple
//! left-to-right shaper. It reads bytes the caller supplies, does no I/O and trusts nothing in them: malformed
//! data is a [`FontError`], never a panic.

mod atlas;
mod font_error;
mod font_face;
mod geometry;
mod glyph_id;
mod outline;
mod raster;
mod reader;
mod shaping;
#[cfg(feature = "synthetic-font")]
pub mod synthetic;
mod tables;

pub use atlas::{AtlasRect, CachedGlyph, GlyphAtlas, GlyphCache};
pub use font_error::FontError;
pub use font_face::FontFace;
pub use geometry::{Contour, Point, Segment};
pub use glyph_id::GlyphId;
pub use outline::Outline;
pub use raster::{Bitmap, rasterize};
pub use shaping::{PlacedGlyph, ShapedRun, shape};
