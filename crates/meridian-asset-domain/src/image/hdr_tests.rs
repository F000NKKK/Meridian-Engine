//! Tests of the Radiance `.hdr` decoder.

use super::*;
use crate::{DecodeError, Decoder};

fn byte(n: usize) -> u8 {
    u8::try_from(n).unwrap()
}

fn assert_close(actual: [f32; 3], expected: [f32; 3]) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!(
            (a - e).abs() <= f32::EPSILON * e.abs().max(1.0),
            "{actual:?} != {expected:?}"
        );
    }
}

fn header(width: usize, height: usize) -> Vec<u8> {
    format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes()
}

fn flat_file(width: usize, quads: &[[u8; 4]]) -> Vec<u8> {
    let mut bytes = header(width, quads.len() / width);
    bytes.extend(quads.iter().flatten());
    bytes
}

/// A new-style RLE scanline: every channel plane is one run of its value.
fn rle_scanline(width: usize, quad: [u8; 4]) -> Vec<u8> {
    let mut bytes = vec![2, 2, byte(width >> 8), byte(width & 0xff)];
    for value in quad {
        let mut left = width;
        while left > 0 {
            let run = left.min(127);
            bytes.extend_from_slice(&[128 + byte(run), value]);
            left -= run;
        }
    }
    bytes
}

#[test]
fn flat_scanlines_decode_to_linear_radiance() {
    let file = flat_file(2, &[[128, 64, 0, 129], [0, 0, 0, 0]]);
    let image = HdrDecoder.decode(&file).unwrap();
    assert_eq!((image.width, image.height), (2, 1));
    // mantissa * 2^(e - 136): 128 * 2^-7 = 1.0, 64 * 2^-7 = 0.5.
    assert_eq!(image.pixels, vec![[1.0, 0.5, 0.0], [0.0, 0.0, 0.0]]);
}

#[test]
fn run_length_encoded_scanlines_decode_per_channel() {
    let mut file = header(300, 2);
    file.extend(rle_scanline(300, [128, 64, 32, 136]));
    file.extend(rle_scanline(300, [1, 2, 3, 128]));
    let image = HdrDecoder.decode(&file).unwrap();
    assert_eq!(image.pixels.len(), 600);
    assert_close(image.pixels[0], [128.0, 64.0, 32.0]);
    assert_close(image.pixels[599], [1.0 / 256.0, 2.0 / 256.0, 3.0 / 256.0]);
}

#[test]
fn literal_runs_inside_rle_scanlines_are_copied() {
    let width = 8;
    let mut file = header(width, 1);
    file.extend_from_slice(&[2, 2, 0, byte(width)]);
    for channel in 0..4u8 {
        file.push(byte(width));
        file.extend((0..byte(width)).map(|x| x + channel));
    }
    let image = HdrDecoder.decode(&file).unwrap();
    assert_eq!(image.pixels.len(), width);
    let scale = (f32::from(4u8) - 136.0).exp2(); // pixel 1: exponent plane holds 1 + 3
    assert_eq!(image.pixels[1][0], 1.0 * scale);
}

#[test]
fn detect_accepts_both_signatures() {
    assert!(HdrDecoder::detect(b"#?RADIANCE\n"));
    assert!(HdrDecoder::detect(b"#?RGBE\n"));
    assert!(!HdrDecoder::detect(b"BM"));
}

#[test]
fn malformed_input_is_a_typed_error_never_a_panic() {
    assert!(matches!(
        HdrDecoder.decode(b"BM...."),
        Err(DecodeError::BadMagic { .. })
    ));
    let cases: [(&[u8], &str); 6] = [
        (
            b"#?RADIANCE\nFORMAT=32-bit_rle_xyze\n\n-Y 1 +X 1\n",
            "format",
        ),
        (
            b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n+Y 1 +X 1\n",
            "orientation",
        ),
        (
            b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 0 +X 1\n",
            "zero height",
        ),
        (
            b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 99999 +X 99999\n",
            "too large",
        ),
        (b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe", "unterminated"),
        (b"#?RADIANCE\n\n-Y 1 +X 1\n\x01\x02", "truncated flat"),
    ];
    for (bytes, what) in cases {
        assert!(HdrDecoder.decode(bytes).is_err(), "{what}");
    }
}

#[test]
fn overrunning_and_truncated_runs_are_rejected() {
    let mut overrun = header(8, 1);
    overrun.extend_from_slice(&[2, 2, 0, 8, 128 + 100, 7]);
    assert!(matches!(
        HdrDecoder.decode(&overrun),
        Err(DecodeError::Malformed(_))
    ));
    let mut truncated = header(8, 1);
    truncated.extend_from_slice(&[2, 2, 0, 8, 128 + 8]);
    assert!(HdrDecoder.decode(&truncated).is_err());
    let mut dump_short = header(8, 1);
    dump_short.extend_from_slice(&[2, 2, 0, 8, 8, 1, 2]);
    assert!(HdrDecoder.decode(&dump_short).is_err());
    let mut zero_run = header(8, 1);
    zero_run.extend_from_slice(&[2, 2, 0, 8, 0]);
    assert!(HdrDecoder.decode(&zero_run).is_err());
}
