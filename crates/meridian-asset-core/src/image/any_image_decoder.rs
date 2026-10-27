//! Decodes any supported image format, chosen by magic bytes.

use super::{BmpDecoder, ImageFormat, PngDecoder};
use crate::{Decoder, ImageAsset};

/// The sniffing front door: [`ImageFormat::detect`](super::ImageFormat::detect) on the bytes, then
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

impl Decoder<ImageAsset> for AnyImageDecoder {
    type Error = crate::DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<ImageAsset, crate::DecodeError> {
        match ImageFormat::detect(bytes) {
            Some(ImageFormat::Bmp) => BmpDecoder.decode(bytes),
            Some(ImageFormat::Png) => PngDecoder.decode(bytes),
            None => Err(crate::DecodeError::Unsupported(
                "unrecognized image signature (not BMP/PNG)",
            )),
        }
    }
}
