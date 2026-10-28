//! [`PackWriter`]: builds a pack in memory.

use super::entry_set::EntrySet;
use super::layout::{HEADER_LEN, IndexLocation, crc32};
use super::{PackBuildError, PackEntry, PackPath};
use crate::AssetId;

/// Collects assets and writes the pack: header, the payloads in the order
/// added, then the index (Substrate Data, binary version 2). Always the
/// newest envelope version. It holds the payloads in memory until
/// [`finish`](Self::finish); for packs larger than memory use
/// [`StreamingPackWriter`](super::StreamingPackWriter), which writes the same
/// bytes.
#[derive(Debug, Default)]
pub struct PackWriter {
    entries: EntrySet,
    payload: Vec<u8>,
}

impl PackWriter {
    /// An empty pack.
    #[must_use]
    pub fn new() -> Self {
        PackWriter::default()
    }

    /// Adds an asset's `bytes` under `id` and `path`.
    ///
    /// # Errors
    ///
    /// [`PackBuildError`] when `id` or `path` is already in the pack;
    /// nothing is added then.
    pub fn add(&mut self, id: AssetId, path: PackPath, bytes: &[u8]) -> Result<(), PackBuildError> {
        self.entries.check(id, &path)?;
        let offset = (HEADER_LEN + self.payload.len()) as u64;
        self.payload.extend_from_slice(bytes);
        self.entries.push(PackEntry {
            id,
            path,
            offset,
            len: bytes.len() as u64,
            crc32: crc32(bytes),
        })
    }

    /// How many assets have been added.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no asset has been added.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }

    /// The pack's bytes.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        let index = self.entries.index_bytes();
        let header = IndexLocation {
            offset: (HEADER_LEN + self.payload.len()) as u64,
            len: index.len() as u64,
            crc32: crc32(&index),
        };
        let mut out = Vec::with_capacity(HEADER_LEN + self.payload.len() + index.len());
        out.extend_from_slice(&header.encode());
        out.extend_from_slice(&self.payload);
        out.extend_from_slice(&index);
        out
    }
}
