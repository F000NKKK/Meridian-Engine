//! Tests of this module.

use super::*;

#[test]
fn detect_identifies_bmp_and_png_and_rejects_unknown() {
    let bmp = crate::image::bmp_decoder::tests_support::make_bmp(1, 1, (1, 2, 3));
    assert_eq!(ImageFormat::detect(&bmp), Some(ImageFormat::Bmp));

    let png_signature = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0];
    assert_eq!(ImageFormat::detect(&png_signature), Some(ImageFormat::Png));

    assert_eq!(
        ImageFormat::detect(&[0xff, 0xd8, 0xff, 0xe0]),
        Some(ImageFormat::Jpeg)
    );
    assert_eq!(ImageFormat::detect(&[0xff, 0xd8]), None);

    assert_eq!(ImageFormat::detect(b"not an image"), None);
    assert_eq!(ImageFormat::detect(&[]), None);
}
