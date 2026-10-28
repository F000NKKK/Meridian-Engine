//! Decoding real JPEG files.

use meridian_asset_domain::{DecodeError, Decoder};

use crate::JpegDecoder;

const RED: &[u8] = include_bytes!("../fixtures/red.jpg");
const GRAY: &[u8] = include_bytes!("../fixtures/gray.jpg");

fn near(actual: u8, expected: u8) -> bool {
    actual.abs_diff(expected) <= 6
}

#[test]
fn a_colour_jpeg_decodes_to_opaque_rgba() {
    let image = JpegDecoder.decode(RED).unwrap();
    assert_eq!((image.width, image.height), (16, 8));
    assert_eq!(image.pixels.len(), 16 * 8 * 4);
    let pixel = &image.pixels[..4];
    assert!(near(pixel[0], 200) && near(pixel[1], 30) && near(pixel[2], 60));
    assert_eq!(pixel[3], 255);
}

#[test]
fn a_grayscale_jpeg_decodes_to_equal_channels() {
    let image = JpegDecoder.decode(GRAY).unwrap();
    assert_eq!((image.width, image.height), (8, 8));
    let pixel = &image.pixels[..4];
    assert!(near(pixel[0], 128) && pixel[0] == pixel[1] && pixel[1] == pixel[2]);
    assert_eq!(pixel[3], 255);
}

#[test]
fn bytes_without_the_marker_are_rejected() {
    assert!(matches!(
        JpegDecoder.decode(b"\x89PNG\r\n\x1a\n"),
        Err(DecodeError::BadMagic { .. })
    ));
    assert!(matches!(
        JpegDecoder.decode(&[0xff]),
        Err(DecodeError::TooShort { .. })
    ));
}

#[test]
fn a_truncated_jpeg_is_an_error_not_a_panic() {
    assert!(JpegDecoder.decode(&RED[..RED.len() / 2]).is_err());
    assert!(JpegDecoder.decode(&RED[..40]).is_err());
}
