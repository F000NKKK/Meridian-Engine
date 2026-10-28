//! Image container formats recognised by their magic bytes.

/// An image format identified from leading magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// `BM` + `BITMAPINFOHEADER` — see [`super::BmpDecoder`].
    Bmp,
    /// The 8-byte PNG signature — see the `PngDecoder` of `meridian-asset-png`.
    Png,
    /// The `FF D8 FF` start-of-image marker — see the `JpegDecoder` of `meridian-asset-jpeg`.
    Jpeg,
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
        if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            return Some(ImageFormat::Jpeg);
        }
        None
    }
}
