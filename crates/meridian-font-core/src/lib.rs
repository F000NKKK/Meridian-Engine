//! Our own font stack, CPU only (see ADR 023). This first slice reads a
//! TrueType face: metrics, character map, kerning and glyph outlines. It
//! reads bytes the caller supplies, does no I/O and trusts nothing in
//! them: malformed data is a [`FontError`], never a panic.

mod font_error;
mod font_face;
mod geometry;
mod glyph_id;
mod outline;
mod reader;
mod tables;

pub use font_error::FontError;
pub use font_face::FontFace;
pub use geometry::{Contour, Point, Segment};
pub use glyph_id::GlyphId;
pub use outline::Outline;
