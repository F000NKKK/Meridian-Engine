//! PNG decoder via the `png` crate — see the `image` barrel module doc
//! and docs/adr/013-compressed-audio-codecs.md for why PNG's DEFLATE
//! payload isn't hand-rolled the way BMP is.

use crate::{DecodeError, Decoder, ImageData, need};

/// Decodes PNG (any color type/bit depth the `png` crate itself
/// supports) into RGBA8, always — palette, grayscale and
/// grayscale+alpha sources are expanded/converted, 16-bit samples are
/// stripped to 8-bit. Detected by the 8-byte PNG signature.
#[derive(Debug, Default)]
pub struct PngDecoder;

impl Decoder<ImageData> for PngDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<ImageData, DecodeError> {
        const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        need(bytes, SIGNATURE.len())?;
        if bytes[..SIGNATURE.len()] != SIGNATURE {
            return Err(DecodeError::BadMagic {
                expected: "\\x89PNG\\r\\n\\x1a\\n",
            });
        }

        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        // Always normalize to 8-bit and force an alpha channel, so the
        // output is either Rgba or GrayscaleAlpha regardless of the
        // source's color type/bit depth/palette — one conversion path
        // below instead of one per PNG color type.
        decoder.set_transformations(
            png::Transformations::EXPAND
                | png::Transformations::STRIP_16
                | png::Transformations::ALPHA,
        );
        let mut reader = decoder
            .read_info()
            .map_err(|e| DecodeError::Codec(e.to_string()))?;

        let mut buffer = vec![0u8; reader.output_buffer_size().unwrap_or(0)];
        let info = reader
            .next_frame(&mut buffer)
            .map_err(|e| DecodeError::Codec(e.to_string()))?;
        let decoded = &buffer[..info.buffer_size()];

        let pixels = match info.color_type {
            png::ColorType::Rgba => decoded.to_vec(),
            png::ColorType::GrayscaleAlpha => decoded
                .chunks_exact(2)
                .flat_map(|ga| [ga[0], ga[0], ga[0], ga[1]])
                .collect(),
            other => {
                return Err(DecodeError::Unsupported(match other {
                    png::ColorType::Grayscale => "PNG grayscale without alpha after normalization",
                    png::ColorType::Rgb => "PNG RGB without alpha after normalization",
                    png::ColorType::Indexed => "PNG indexed color after normalization",
                    _ => "unrecognized PNG color type after normalization",
                }));
            }
        };

        Ok(ImageData {
            width: info.width,
            height: info.height,
            pixels,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Encodes a real PNG via the same `png` crate this decoder wraps —
    /// hand-building valid DEFLATE-compressed bytes isn't practical the
    /// way hand-building BMP/OBJ bytes is (see the module doc's
    /// DEFLATE-is-too-big-to-hand-roll reasoning); this exercises *our*
    /// wiring and color-type conversion, not the `png` crate itself.
    fn make_png(width: u32, height: u32, color: png::ColorType, pixels: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixels).unwrap();
        writer.finish().unwrap();
        bytes
    }

    #[test]
    fn png_decodes_rgba_directly() {
        let pixels = [
            10, 20, 30, 255, 40, 50, 60, 128, 70, 80, 90, 0, 100, 110, 120, 255,
        ];
        let bytes = make_png(2, 2, png::ColorType::Rgba, &pixels);
        let image = PngDecoder.decode(&bytes).unwrap();
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(image.pixels, pixels);
    }

    #[test]
    fn png_expands_rgb_to_opaque_rgba() {
        let pixels = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let bytes = make_png(2, 2, png::ColorType::Rgb, &pixels);
        let image = PngDecoder.decode(&bytes).unwrap();
        assert_eq!(
            image.pixels,
            vec![
                10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255
            ]
        );
    }

    #[test]
    fn png_expands_grayscale_to_rgba() {
        let pixels = [0u8, 128, 255, 64];
        let bytes = make_png(2, 2, png::ColorType::Grayscale, &pixels);
        let image = PngDecoder.decode(&bytes).unwrap();
        assert_eq!(
            image.pixels,
            vec![
                0, 0, 0, 255, 128, 128, 128, 255, 255, 255, 255, 255, 64, 64, 64, 255
            ]
        );
    }

    #[test]
    fn png_rejects_bad_magic() {
        assert!(matches!(
            PngDecoder.decode(b"not a png file at all!!"),
            Err(DecodeError::BadMagic { .. })
        ));
    }

    #[test]
    fn png_rejects_truncated_buffer() {
        assert!(matches!(
            PngDecoder.decode(&[0x89, b'P', b'N']),
            Err(DecodeError::TooShort { .. })
        ));
    }
}
