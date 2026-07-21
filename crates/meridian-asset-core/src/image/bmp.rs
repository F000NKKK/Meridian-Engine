//! Hand-rolled uncompressed BMP decoder — simple enough to own outright,
//! unlike PNG (see `png_decoder`).

use crate::{DecodeError, Decoder, ImageData, need, u16_le, u32_le};

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

impl Decoder<ImageData> for BmpDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<ImageData, DecodeError> {
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

        let row_stride = (width as usize * src_bytes_per_pixel).div_ceil(4) * 4; // rows padded to 4 bytes
        need(bytes, pixel_offset + row_stride * height as usize)?;

        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        for y in 0..height as usize {
            // BMP rows are bottom-to-top unless the height field was negative.
            let src_row = if top_down { y } else { height as usize - 1 - y };
            let row_start = pixel_offset + src_row * row_stride;
            for x in 0..width as usize {
                let src = row_start + x * src_bytes_per_pixel;
                let dst = (y * width as usize + x) * 4;
                // BMP stores B,G,R(,A); we produce R,G,B,A.
                pixels[dst] = bytes[src + 2];
                pixels[dst + 1] = bytes[src + 1];
                pixels[dst + 2] = bytes[src];
                pixels[dst + 3] = if src_bytes_per_pixel == 4 {
                    bytes[src + 3]
                } else {
                    255
                };
            }
        }

        Ok(ImageData {
            width,
            height,
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
    use super::*;
    use tests_support::make_bmp;

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
