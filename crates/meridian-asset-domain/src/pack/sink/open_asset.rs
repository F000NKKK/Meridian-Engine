//! The asset a streaming writer is in the middle of.

use crate::AssetId;
use crate::pack::PackPath;
use crate::pack::layout::Crc32;

/// An asset between `begin` and `end`: where it started and what has
/// been written so far.
#[derive(Debug)]
pub(super) struct OpenAsset {
    pub(super) id: AssetId,
    pub(super) path: PackPath,
    pub(super) offset: u64,
    pub(super) len: u64,
    pub(super) crc: Crc32,
}
