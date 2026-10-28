//! `data:` URIs: the only way a `.gltf` carries bytes without a second file.

use crate::GltfError;

/// The bytes of a `data:[<mime>];base64,<payload>` URI. A URI with any
/// other scheme is an external resource.
pub(crate) fn decode(uri: &str) -> Result<Vec<u8>, GltfError> {
    let Some(rest) = uri.strip_prefix("data:") else {
        return Err(GltfError::ExternalResource(uri.to_owned()));
    };
    let (header, payload) = rest.split_once(',').ok_or(GltfError::BadDataUri)?;
    if !header.ends_with(";base64") {
        return Err(GltfError::BadDataUri);
    }
    base64(payload.as_bytes())
}

/// Standard-alphabet base64 with optional `=` padding, rejecting any other
/// byte. The output holds at most three bytes per four input bytes.
fn base64(input: &[u8]) -> Result<Vec<u8>, GltfError> {
    let trimmed = match input {
        [rest @ .., b'=', b'='] | [rest @ .., b'='] | rest => rest,
    };
    let mut out = Vec::with_capacity(trimmed.len() / 4 * 3 + 2);
    let mut accumulator = 0u32;
    let mut bits = 0u32;
    for &byte in trimmed {
        accumulator = (accumulator << 6) | sextet(byte)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(
                u8::try_from((accumulator >> bits) & 0xff).map_err(|_| GltfError::BadDataUri)?,
            );
            accumulator &= (1 << bits) - 1;
        }
    }
    if trimmed.len() % 4 == 1 {
        return Err(GltfError::BadDataUri);
    }
    Ok(out)
}

fn sextet(byte: u8) -> Result<u32, GltfError> {
    match byte {
        b'A'..=b'Z' => Ok(u32::from(byte - b'A')),
        b'a'..=b'z' => Ok(u32::from(byte - b'a') + 26),
        b'0'..=b'9' => Ok(u32::from(byte - b'0') + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(GltfError::BadDataUri),
    }
}
