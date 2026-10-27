//! Tests of this module.

use super::*;
use crate::Decoder;

#[test]
fn detect_identifies_bmp_and_png_and_rejects_unknown() {
    let bmp = crate::image::bmp::tests_support::make_bmp(1, 1, (1, 2, 3));
    assert_eq!(ImageFormat::detect(&bmp), Some(ImageFormat::Bmp));

    let png =
        crate::image::png_decoder::tests_support::make_png(1, 1, png::ColorType::Rgb, &[1, 2, 3]);
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
