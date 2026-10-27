//! Our own font stack, CPU only (see ADR 023). This first slice reads a
//! TrueType face: metrics, character map, kerning and glyph outlines. It
//! reads bytes the caller supplies, does no I/O and trusts nothing in
//! them: malformed data is a [`FontError`], never a panic.

mod advances;
mod cmap;
mod contour;
mod directory;
mod error;
mod face;
mod glyf;
mod glyph_id;
mod head;
mod kerning;
mod loca;
mod outline;
mod point;
mod reader;
mod segment;

pub use contour::Contour;
pub use error::FontError;
pub use face::FontFace;
pub use glyph_id::GlyphId;
pub use outline::Outline;
pub use point::Point;
pub use segment::Segment;
