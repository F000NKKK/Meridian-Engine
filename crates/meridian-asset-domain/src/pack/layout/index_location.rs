//! [`IndexLocation`]: what the fixed 32-byte header of a pack says.
//!
//! ```text
//!  0  magic          "MPAK"
//!  4  version        u16   envelope version (PACK_VERSION when written)
//!  6  flags          u16   0 in version 1
//!  8  index_offset   u64   where the index starts, from the start of the file
//! 16  index_len      u64   its length; offset + length is the end of the file
//! 24  index_crc32    u32   CRC-32 of the index bytes
//! 28  header_crc32   u32   CRC-32 of bytes 0..28
//! ```
//!
//! All numbers are little-endian. The payloads follow the header
//! back to back, then the index.

use super::checksum::crc32;
use super::header::HEADER_LEN;
use super::{PACK_MAGIC, PACK_VERSION};
use crate::pack::PackError;

/// The bytes the header checksum covers.
const CHECKED_LEN: usize = 28;

/// What the header says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::pack) struct IndexLocation {
    pub(in crate::pack) offset: u64,
    pub(in crate::pack) len: u64,
    pub(in crate::pack) crc32: u32,
}

impl IndexLocation {
    pub(in crate::pack) fn encode(&self) -> [u8; HEADER_LEN] {
        let mut out = [0_u8; HEADER_LEN];
        out[0..4].copy_from_slice(&PACK_MAGIC);
        out[4..6].copy_from_slice(&PACK_VERSION.to_le_bytes());
        // flags (6..8) stay 0.
        out[8..16].copy_from_slice(&self.offset.to_le_bytes());
        out[16..24].copy_from_slice(&self.len.to_le_bytes());
        out[24..28].copy_from_slice(&self.crc32.to_le_bytes());
        let checksum = crc32(&out[..CHECKED_LEN]);
        out[28..32].copy_from_slice(&checksum.to_le_bytes());
        out
    }

    /// Reads the header of `bytes` (a file's first 32): magic, then
    /// version, then the checksum, then the flags, in that order, so a file
    /// of another kind or a newer version is named as such.
    pub(in crate::pack) fn decode(bytes: &[u8; HEADER_LEN]) -> Result<IndexLocation, PackError> {
        if bytes[0..4] != PACK_MAGIC {
            return Err(PackError::BadMagic);
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version == 0 || version > PACK_VERSION {
            return Err(PackError::UnsupportedVersion { found: version });
        }
        let stored = u32_at(bytes, 28);
        let computed = crc32(&bytes[..CHECKED_LEN]);
        if stored != computed {
            return Err(PackError::HeaderChecksum { stored, computed });
        }
        let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
        if flags != 0 {
            return Err(PackError::UnknownFlags { flags });
        }
        Ok(IndexLocation {
            offset: u64_at(bytes, 8),
            len: u64_at(bytes, 16),
            crc32: u32_at(bytes, 24),
        })
    }
}

fn u32_at(bytes: &[u8; HEADER_LEN], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn u64_at(bytes: &[u8; HEADER_LEN], at: usize) -> u64 {
    let mut eight = [0_u8; 8];
    eight.copy_from_slice(&bytes[at..at + 8]);
    u64::from_le_bytes(eight)
}
