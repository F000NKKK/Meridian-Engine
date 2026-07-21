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

pub mod bmp;
pub mod png_decoder;

pub use bmp::BmpDecoder;
pub use png_decoder::PngDecoder;

use crate::{Decoder, ImageData};

/// An image format identified from leading magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// `BM` + `BITMAPINFOHEADER` — see [`BmpDecoder`].
    Bmp,
    /// The 8-byte PNG signature — see [`PngDecoder`].
    Png,
}

impl ImageFormat {
    /// Identifies the format from the buffer's leading bytes. Returns
    /// `None` when no known signature matches — extension-based
    /// guessing is deliberately not a fallback (see the module doc).
    pub fn detect(bytes: &[u8]) -> Option<ImageFormat> {
        if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
            return Some(ImageFormat::Bmp);
        }
        const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        if bytes.len() >= PNG_SIGNATURE.len() && bytes[..PNG_SIGNATURE.len()] == PNG_SIGNATURE {
            return Some(ImageFormat::Png);
        }
        None
    }
}

/// The sniffing front door: [`ImageFormat::detect`] on the bytes, then
/// dispatch to the matching decoder — the same shape
/// [`crate::audio::AnyAudioDecoder`] gives audio formats.
#[derive(Debug, Default)]
pub struct AnyImageDecoder;

impl AnyImageDecoder {
    /// The detected format for `bytes`, if any.
    pub fn sniff(&self, bytes: &[u8]) -> Option<ImageFormat> {
        ImageFormat::detect(bytes)
    }
}

impl Decoder<ImageData> for AnyImageDecoder {
    type Error = crate::DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<ImageData, crate::DecodeError> {
        match ImageFormat::detect(bytes) {
            Some(ImageFormat::Bmp) => BmpDecoder.decode(bytes),
            Some(ImageFormat::Png) => PngDecoder.decode(bytes),
            None => Err(crate::DecodeError::Unsupported(
                "unrecognized image signature (not BMP/PNG)",
            )),
        }
    }
}

#[cfg(test)]
mod any_image_decoder_tests {
    use super::*;

    #[test]
    fn detect_identifies_bmp_and_png_and_rejects_unknown() {
        let bmp = crate::image::bmp::tests_support::make_bmp(1, 1, (1, 2, 3));
        assert_eq!(ImageFormat::detect(&bmp), Some(ImageFormat::Bmp));

        let png = crate::image::png_decoder::tests_support::make_png(
            1,
            1,
            png::ColorType::Rgb,
            &[1, 2, 3],
        );
        assert_eq!(ImageFormat::detect(&png), Some(ImageFormat::Png));

        assert_eq!(ImageFormat::detect(b"not an image"), None);
        assert_eq!(ImageFormat::detect(&[]), None);
    }

    #[test]
    fn any_image_decoder_dispatches_to_the_right_decoder() {
        let bmp = crate::image::bmp::tests_support::make_bmp(2, 1, (10, 20, 30));
        let decoded = AnyImageDecoder.decode(&bmp).unwrap();
        assert_eq!((decoded.width, decoded.height), (2, 1));

        let png = crate::image::png_decoder::tests_support::make_png(
            2,
            1,
            png::ColorType::Rgba,
            &[1, 2, 3, 255, 4, 5, 6, 255],
        );
        let decoded = AnyImageDecoder.decode(&png).unwrap();
        assert_eq!((decoded.width, decoded.height), (2, 1));

        assert!(AnyImageDecoder.decode(b"nonsense").is_err());
    }
}
