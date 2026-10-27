//! Packing glyph bitmaps into one texture page, and remembering where.

mod atlas_rect;
mod cached_glyph;
mod glyph_atlas;
mod glyph_cache;
mod shelf;

pub use atlas_rect::AtlasRect;
pub use cached_glyph::CachedGlyph;
pub use glyph_atlas::GlyphAtlas;
pub use glyph_cache::GlyphCache;
