//! Image decoding that needs no codec library:
//!
//! - [`bmp_decoder`]: hand-rolled uncompressed BMP.
//! - [`format`]: [`ImageFormat::detect`] identifies a format from its
//!   leading magic bytes, never a file extension — the same rule
//!   [`crate::audio::AudioFormat::detect`] follows for audio.
//!
//! PNG (DEFLATE is too big to hand-roll, ADR 013) is the adapter crate
//! `meridian-asset-png` and JPEG is `meridian-asset-jpeg`; the composition root picks the decoder for a
//! detected format.

pub mod bmp_decoder;
mod format;
#[cfg(test)]
mod tests;

pub use bmp_decoder::BmpDecoder;
pub use format::ImageFormat;
