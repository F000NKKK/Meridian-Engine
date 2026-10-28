//! JPEG decoder via the `zune-jpeg` crate.

use meridian_asset_domain::{DecodeError, Decoder, ImageAsset};
use zune_jpeg::JpegDecoder as ZuneDecoder;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

/// The largest side a decoded image may have; a header claiming more is
/// refused before any pixel memory is allocated.
const MAX_SIDE: usize = 16_384;

/// Decodes baseline and progressive JPEG into RGBA8, always opaque.
/// Grayscale, YCbCr and CMYK sources are converted by the codec; detected
/// by the `FF D8 FF` start-of-image marker.
#[derive(Debug, Default)]
pub struct JpegDecoder;

impl Decoder<ImageAsset> for JpegDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<ImageAsset, DecodeError> {
        const MARKER: [u8; 3] = [0xff, 0xd8, 0xff];
        if bytes.len() < MARKER.len() {
            return Err(DecodeError::TooShort {
                needed: MARKER.len(),
                got: bytes.len(),
            });
        }
        if bytes[..MARKER.len()] != MARKER {
            return Err(DecodeError::BadMagic {
                expected: "\\xff\\xd8\\xff",
            });
        }
        let options = DecoderOptions::default()
            .jpeg_set_out_colorspace(ColorSpace::RGBA)
            .set_max_width(MAX_SIDE)
            .set_max_height(MAX_SIDE);
        let mut decoder = ZuneDecoder::new_with_options(ZCursor::new(bytes), options);
        let pixels = decoder
            .decode()
            .map_err(|err| DecodeError::Codec(format!("{err:?}")))?;
        let info = decoder
            .info()
            .ok_or(DecodeError::Malformed("JPEG without a frame header"))?;
        let (width, height) = (u32::from(info.width), u32::from(info.height));
        if pixels.len() != width as usize * height as usize * 4 {
            return Err(DecodeError::Malformed("JPEG pixel count mismatch"));
        }
        Ok(ImageAsset {
            width,
            height,
            pixels,
        })
    }
}
