//! Image decoding, one file per format (barrel module):
//!
//! - [`bmp`]: hand-rolled uncompressed BMP.
//! - [`png_decoder`]: PNG via the `png` crate (ADR 013's
//!   DEFLATE-is-too-big-to-hand-roll reasoning applies identically to
//!   PNG's zlib-compressed data).
//!
//! [`ImageFormat::detect`]/[`AnyImageDecoder`] identify a format from its
//! leading magic bytes, never a file extension — the same rule
//! [`crate::audio::AudioFormat::detect`] follows for audio. JPEG is
//! future work on the same when-a-concrete-asset-needs-it trigger; a
//! future decoder gets its own file here, a re-export below, and a new
//! [`ImageFormat`] variant.

mod any_image_decoder;
pub mod bmp;
mod image_format;
pub mod png_decoder;
#[cfg(test)]
mod tests;

pub use any_image_decoder::AnyImageDecoder;
pub use bmp::BmpDecoder;
pub use image_format::ImageFormat;
pub use png_decoder::PngDecoder;
