//! Bounds-checked little-endian reads from a byte slice.

use crate::DecodeError;

pub(crate) fn need(bytes: &[u8], len: usize) -> Result<(), DecodeError> {
    if bytes.len() < len {
        Err(DecodeError::TooShort {
            needed: len,
            got: bytes.len(),
        })
    } else {
        Ok(())
    }
}

pub(crate) fn u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

pub(crate) fn u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
