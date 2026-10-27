//! Hand-rolled uncompressed BMP decoder — simple enough to own outright,
//! unlike PNG (see `png_decoder`).

use crate::{DecodeError, Decoder, ImageAsset, need, u16_le, u32_le};

fn i32_le(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Decodes uncompressed 24-bit or 32-bit BMP (`BITMAPINFOHEADER`, `BI_RGB`
/// only — no RLE compression, no palette/indexed color).
#[derive(Debug, Default)]
pub struct BmpDecoder;

/// Where a BMP's pixel rows sit in the file.
struct PixelLayout {
    offset: usize,
    width: usize,
    height: usize,
    top_down: bool,
    bytes_per_pixel: usize,
}

impl PixelLayout {
    /// Validates the headers and locates the pixel rows.
    fn parse(bytes: &[u8]) -> Result<Self, DecodeError> {
        need(bytes, 54)?; // 14-byte file header + 40-byte BITMAPINFOHEADER
        if &bytes[0..2] != b"BM" {
            return Err(DecodeError::BadMagic { expected: "BM" });
        }
        let pixel_offset = u32_le(bytes, 10) as usize;
        let dib_header_size = u32_le(bytes, 14);
        if dib_header_size < 40 {
            return Err(DecodeError::Unsupported(
                "DIB header smaller than BITMAPINFOHEADER",
            ));
        }
        let width = i32_le(bytes, 18);
        let height_raw = i32_le(bytes, 22);
        let bits_per_pixel = u16_le(bytes, 28);
        let compression = u32_le(bytes, 30);

        if compression != 0 {
            return Err(DecodeError::Unsupported(
                "BMP compression other than BI_RGB",
            ));
        }
        if width <= 0 {
            return Err(DecodeError::Malformed("non-positive width"));
        }
        let (height, top_down) = if height_raw < 0 {
            (-height_raw, true)
        } else {
            (height_raw, false)
        };
        if height <= 0 {
            return Err(DecodeError::Malformed("zero height"));
        }
        let (width, height) = (width as u32, height as u32);
        let src_bytes_per_pixel = match bits_per_pixel {
            24 => 3,
            32 => 4,
            _ => return Err(DecodeError::Unsupported("BMP bit depth other than 24/32")),
        };

        Ok(Self {
            offset: pixel_offset,
            width: width as usize,
            height: height as usize,
            top_down,
            bytes_per_pixel: src_bytes_per_pixel,
        })
    }

    /// Rows are padded to 4 bytes.
    fn row_stride(&self) -> usize {
        (self.width * self.bytes_per_pixel).div_ceil(4) * 4
    }

    /// Converts the file's B,G,R(,A) rows into top-to-bottom R,G,B,A.
    fn to_rgba(&self, bytes: &[u8]) -> Vec<u8> {
        let stride = self.row_stride();
        let mut pixels = vec![0u8; self.width * self.height * 4];
        for y in 0..self.height {
            // BMP rows are bottom-to-top unless the height field was negative.
            let src_row = if self.top_down {
                y
            } else {
                self.height - 1 - y
            };
            let row_start = self.offset + src_row * stride;
            for x in 0..self.width {
                let src = row_start + x * self.bytes_per_pixel;
                let dst = (y * self.width + x) * 4;
                pixels[dst] = bytes[src + 2];
                pixels[dst + 1] = bytes[src + 1];
                pixels[dst + 2] = bytes[src];
                pixels[dst + 3] = if self.bytes_per_pixel == 4 {
                    bytes[src + 3]
                } else {
                    255
                };
            }
        }
        pixels
    }
}

impl Decoder<ImageAsset> for BmpDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<ImageAsset, DecodeError> {
        let layout = PixelLayout::parse(bytes)?;
        need(bytes, layout.offset + layout.row_stride() * layout.height)?;
        let pixels = layout.to_rgba(bytes);

        Ok(ImageAsset {
            width: u32::try_from(layout.width)
                .map_err(|_| DecodeError::Malformed("width too large"))?,
            height: u32::try_from(layout.height)
                .map_err(|_| DecodeError::Malformed("height too large"))?,
            pixels,
        })
    }
}
/// Test-fixture builder shared with `image::mod`'s `AnyImageDecoder`
/// tests — `pub(crate)` rather than duplicated, since a real BMP file
/// (unlike a PNG) is simple enough to hand-build but still shouldn't be
/// built twice.
#[cfg(test)]
pub(crate) mod tests_support {
    /// Builds a minimal uncompressed 24-bit BMP: `width`x`height`, every
    /// pixel the given (r,g,b).
    pub(crate) fn make_bmp(width: u32, height: u32, rgb: (u8, u8, u8)) -> Vec<u8> {
        let row_stride = (width as usize * 3).div_ceil(4) * 4;
        let pixel_data_len = row_stride * height as usize;
        let file_size = 54 + pixel_data_len;

        let mut b = Vec::with_capacity(file_size);
        b.extend_from_slice(b"BM");
        b.extend_from_slice(&(file_size as u32).to_le_bytes());
        b.extend_from_slice(&[0, 0, 0, 0]); // reserved
        b.extend_from_slice(&54u32.to_le_bytes()); // pixel data offset
        b.extend_from_slice(&40u32.to_le_bytes()); // DIB header size
        b.extend_from_slice(&(width as i32).to_le_bytes());
        b.extend_from_slice(&(height as i32).to_le_bytes()); // positive -> bottom-up
        b.extend_from_slice(&1u16.to_le_bytes()); // planes
        b.extend_from_slice(&24u16.to_le_bytes()); // bits per pixel
        b.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
        b.extend_from_slice(&(pixel_data_len as u32).to_le_bytes());
        b.extend_from_slice(&2835i32.to_le_bytes()); // x pixels/meter
        b.extend_from_slice(&2835i32.to_le_bytes()); // y pixels/meter
        b.extend_from_slice(&0u32.to_le_bytes()); // colors used
        b.extend_from_slice(&0u32.to_le_bytes()); // important colors

        for _ in 0..height {
            for _ in 0..width {
                b.push(rgb.2); // B
                b.push(rgb.1); // G
                b.push(rgb.0); // R
            }
            b.extend(std::iter::repeat_n(0u8, row_stride - width as usize * 3)); // row padding
        }
        b
    }
}

#[cfg(test)]
mod tests {
    use tests_support::make_bmp;

    use super::*;

    #[test]
    fn bmp_decodes_solid_color_and_dimensions() {
        let bytes = make_bmp(3, 2, (10, 20, 30));
        let image = BmpDecoder.decode(&bytes).unwrap();
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!(image.pixels.len(), 3 * 2 * 4);
        for px in image.pixels.chunks_exact(4) {
            assert_eq!(px, &[10, 20, 30, 255]);
        }
    }

    #[test]
    fn bmp_row_padding_and_bottom_up_order_are_handled() {
        // width=1 forces row padding (3 bytes -> padded to 4); make row 0
        // (bottom, stored first) a different color from the top row to
        // verify vertical flip.
        let mut b = make_bmp(1, 2, (0, 0, 0));
        // Overwrite bottom row (first in file) to red, top row (second in
        // file) stays black, so decoded row 0 (top of image) must be black
        // and row 1 (bottom) must be red.
        let pixel_start = 54;
        b[pixel_start] = 0; // B
        b[pixel_start + 1] = 0; // G
        b[pixel_start + 2] = 255; // R

        let image = BmpDecoder.decode(&b).unwrap();
        let top_row_pixel = &image.pixels[0..4];
        let bottom_row_pixel = &image.pixels[4..8];
        assert_eq!(
            top_row_pixel,
            &[0, 0, 0, 255],
            "top row must come from the last row in the file"
        );
        assert_eq!(
            bottom_row_pixel,
            &[255, 0, 0, 255],
            "bottom row must come from the first row in the file"
        );
    }

    #[test]
    fn bmp_rejects_bad_magic() {
        let mut bytes = make_bmp(1, 1, (0, 0, 0));
        bytes[0] = b'X';
        assert_eq!(
            BmpDecoder.decode(&bytes),
            Err(DecodeError::BadMagic { expected: "BM" })
        );
    }

    #[test]
    fn bmp_rejects_truncated_buffer() {
        assert!(matches!(
            BmpDecoder.decode(&[0u8; 10]),
            Err(DecodeError::TooShort { .. })
        ));
    }
}
