//! Image decoding, one file per format (barrel module):
//!
//! - [`bmp`]: hand-rolled uncompressed BMP.
//! - [`png_decoder`]: PNG via the `png` crate (ADR 013's
//!   DEFLATE-is-too-big-to-hand-roll reasoning applies identically to
//!   PNG's zlib-compressed data).
//!
//! JPEG is future work on the same when-a-concrete-asset-needs-it
//! trigger; a future decoder gets its own file here and a re-export
//! below, the same shape [`crate::audio`] uses for audio formats.

pub mod bmp;
pub mod png_decoder;

pub use bmp::BmpDecoder;
pub use png_decoder::PngDecoder;
