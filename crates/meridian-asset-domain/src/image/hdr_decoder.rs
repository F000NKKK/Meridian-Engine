//! Radiance RGBE (`.hdr`) decoder: the format environment maps ship in.

use crate::{DecodeError, Decoder, HdrImageAsset};

/// Largest accepted image, in pixels (a 8192 × 4096 panorama fits twice).
const MAX_PIXELS: u64 = 1 << 26;
/// Scanline widths the new-style run-length encoding can represent.
const RLE_WIDTHS: std::ops::Range<usize> = 8..32768;

/// Decodes Radiance `.hdr` files: the `#?RADIANCE` / `#?RGBE` header, a
/// `-Y height +X width` resolution line, and flat or run-length-encoded
/// RGBE scanlines. Other orientations are refused, not guessed.
#[derive(Debug, Default)]
pub struct HdrDecoder;

impl HdrDecoder {
    /// Whether `bytes` start with a Radiance signature.
    #[must_use]
    pub fn detect(bytes: &[u8]) -> bool {
        bytes.starts_with(b"#?RADIANCE") || bytes.starts_with(b"#?RGBE")
    }
}

impl Decoder<HdrImageAsset> for HdrDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<HdrImageAsset, DecodeError> {
        if !Self::detect(bytes) {
            return Err(DecodeError::BadMagic {
                expected: "#?RADIANCE",
            });
        }
        let (width, height, mut body) = parse_header(bytes)?;
        let mut pixels = Vec::new();
        let mut scanline = vec![[0u8; 4]; width];
        for _ in 0..height {
            body = read_scanline(body, &mut scanline)?;
            pixels.extend(scanline.iter().copied().map(rgbe_to_linear));
        }
        // parse_header capped the pixel count, so both fit a `u32`.
        let to_u32 =
            |n: usize| u32::try_from(n).map_err(|_| DecodeError::Malformed("HDR dimension"));
        Ok(HdrImageAsset {
            width: to_u32(width)?,
            height: to_u32(height)?,
            pixels,
        })
    }
}

/// Splits the file into `(width, height, pixel bytes)`.
fn parse_header(bytes: &[u8]) -> Result<(usize, usize, &[u8]), DecodeError> {
    let mut rest = bytes;
    let mut format_ok = true;
    loop {
        let (line, tail) = next_line(rest)?;
        rest = tail;
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix(b"FORMAT=") {
            format_ok = value == b"32-bit_rle_rgbe";
        }
    }
    if !format_ok {
        return Err(DecodeError::Unsupported(
            "HDR format other than 32-bit_rle_rgbe",
        ));
    }
    let (line, body) = next_line(rest)?;
    let (width, height) = parse_resolution(line)?;
    if (width as u64) * (height as u64) > MAX_PIXELS {
        return Err(DecodeError::Unsupported(
            "HDR image larger than 2^26 pixels",
        ));
    }
    Ok((width, height, body))
}

/// One `\n`-terminated line (without the terminator) and what follows it.
fn next_line(bytes: &[u8]) -> Result<(&[u8], &[u8]), DecodeError> {
    let end = bytes
        .iter()
        .position(|&b| b == b'\n')
        .ok_or(DecodeError::Malformed("HDR header is not terminated"))?;
    Ok((&bytes[..end], &bytes[end + 1..]))
}

/// Parses `-Y <height> +X <width>`.
fn parse_resolution(line: &[u8]) -> Result<(usize, usize), DecodeError> {
    let text = std::str::from_utf8(line)
        .map_err(|_| DecodeError::Malformed("HDR resolution line is not text"))?;
    let mut parts = text.split_whitespace();
    let (Some("-Y"), Some(h), Some("+X"), Some(w), None) = (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) else {
        return Err(DecodeError::Unsupported("HDR orientation other than -Y +X"));
    };
    let parse = |s: &str| {
        s.parse::<usize>()
            .ok()
            .filter(|&n| n > 0)
            .ok_or(DecodeError::Malformed(
                "HDR dimension is not a positive number",
            ))
    };
    Ok((parse(w)?, parse(h)?))
}

/// Fills `out` from the front of `body` and returns the remaining bytes.
fn read_scanline<'a>(body: &'a [u8], out: &mut [[u8; 4]]) -> Result<&'a [u8], DecodeError> {
    let width = out.len();
    let is_rle = RLE_WIDTHS.contains(&width)
        && body.len() >= 4
        && body[0] == 2
        && body[1] == 2
        && ((usize::from(body[2]) << 8) | usize::from(body[3])) == width;
    if !is_rle {
        let len = width * 4;
        let flat = body.get(..len).ok_or(DecodeError::TooShort {
            needed: len,
            got: body.len(),
        })?;
        for (texel, quad) in out.iter_mut().zip(flat.chunks_exact(4)) {
            texel.copy_from_slice(quad);
        }
        return Ok(&body[len..]);
    }
    let mut rest = &body[4..];
    for channel in 0..4 {
        rest = read_rle_channel(rest, out, channel)?;
    }
    Ok(rest)
}

/// Decodes one run-length-encoded channel plane of a scanline.
fn read_rle_channel<'a>(
    mut rest: &'a [u8],
    out: &mut [[u8; 4]],
    channel: usize,
) -> Result<&'a [u8], DecodeError> {
    let mut x = 0;
    while x < out.len() {
        let (&count, tail) = rest
            .split_first()
            .ok_or(DecodeError::Malformed("HDR scanline ends early"))?;
        let is_run = count > 128;
        let len = usize::from(if is_run { count - 128 } else { count });
        if len == 0 || x + len > out.len() {
            return Err(DecodeError::Malformed("HDR run overruns its scanline"));
        }
        rest = if is_run {
            let (&value, tail) = tail
                .split_first()
                .ok_or(DecodeError::Malformed("HDR scanline ends early"))?;
            out[x..x + len].iter_mut().for_each(|t| t[channel] = value);
            tail
        } else {
            let dump = tail
                .get(..len)
                .ok_or(DecodeError::Malformed("HDR scanline ends early"))?;
            for (texel, &value) in out[x..x + len].iter_mut().zip(dump) {
                texel[channel] = value;
            }
            &tail[len..]
        };
        x += len;
    }
    Ok(rest)
}

/// `mantissa * 2^(exponent - 136)`; exponent 0 is black.
fn rgbe_to_linear([r, g, b, e]: [u8; 4]) -> [f32; 3] {
    if e == 0 {
        return [0.0; 3];
    }
    let scale = (f32::from(e) - 136.0).exp2();
    [
        f32::from(r) * scale,
        f32::from(g) * scale,
        f32::from(b) * scale,
    ]
}
