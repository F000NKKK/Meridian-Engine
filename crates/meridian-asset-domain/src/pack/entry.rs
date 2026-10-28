//! [`PackEntry`]: one asset's place in a pack.

use super::PackPath;
use crate::AssetId;

/// Where one asset's bytes are in a pack, and what they must sum to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackEntry {
    /// The asset's stable id.
    pub id: AssetId,
    /// The asset's path in the pack.
    pub path: PackPath,
    /// Where the bytes start, from the start of the file.
    pub offset: u64,
    /// How many bytes.
    pub len: u64,
    /// The CRC-32 of the bytes.
    pub crc32: u32,
}
