//! Decodes any image format the engine ships, chosen by magic bytes.

use meridian_asset_domain::{BmpDecoder, DecodeError, Decoder, ImageAsset, ImageFormat};
use meridian_asset_jpeg::JpegDecoder;
use meridian_asset_png::PngDecoder;

/// The sniffing front door for images: [`ImageFormat::detect`] on the
/// bytes, then the matching decoder. It lives in the composition root
/// because choosing which adapters (`PngDecoder`) are wired in is the
/// root's job, not the domain's.
#[derive(Debug, Default)]
pub struct AnyImageDecoder;

impl AnyImageDecoder {
    /// The detected format for `bytes`, if any.
    #[must_use]
    pub fn sniff(&self, bytes: &[u8]) -> Option<ImageFormat> {
        ImageFormat::detect(bytes)
    }
}

impl Decoder<ImageAsset> for AnyImageDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<ImageAsset, DecodeError> {
        match ImageFormat::detect(bytes) {
            Some(ImageFormat::Bmp) => BmpDecoder.decode(bytes),
            Some(ImageFormat::Png) => PngDecoder.decode(bytes),
            Some(ImageFormat::Jpeg) => JpegDecoder.decode(bytes),
            None => Err(DecodeError::Unsupported(
                "unrecognized image signature (not BMP/PNG/JPEG)",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_png(width: u32, color: png::ColorType, pixels: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, 1);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixels).unwrap();
        writer.finish().unwrap();
        bytes
    }

    #[test]
    fn dispatches_png_to_the_png_adapter() {
        let png = make_png(2, png::ColorType::Rgba, &[1, 2, 3, 255, 4, 5, 6, 255]);
        let decoded = AnyImageDecoder.decode(&png).unwrap();
        assert_eq!((decoded.width, decoded.height), (2, 1));
        assert_eq!(AnyImageDecoder.sniff(&png), Some(ImageFormat::Png));
    }

    #[test]
    fn dispatches_bmp_to_the_domain_decoder() {
        let mut bmp = vec![0u8; 54 + 4];
        bmp[0..2].copy_from_slice(b"BM");
        assert_eq!(AnyImageDecoder.sniff(&bmp), Some(ImageFormat::Bmp));
    }

    #[test]
    fn rejects_unknown_signatures() {
        assert!(AnyImageDecoder.decode(b"nonsense").is_err());
    }
}
