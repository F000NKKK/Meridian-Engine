//! [`PackReader`]: random access to the assets of a pack.

use std::collections::HashMap;

use super::open::{
    check_entries, decode_index, lies_in_payload, read_exact, read_header, read_index,
};
use crate::AssetId;
use crate::pack::layout::{HEADER_LEN, crc32};
use crate::pack::{PackEntry, PackError, PackPath, ReadAt};

/// The largest index this reads, in bytes. A hostile header cannot make
/// `open` allocate more than this, nor more than the file holds.
pub const MAX_INDEX_LEN: u64 = 256 * 1024 * 1024;

/// An open pack. Opening reads the header and the index and nothing else;
/// an asset's bytes are read, and checked against their checksum, when it
/// is asked for. Immutable once open, so one reader serves any number of
/// threads when its source does.
#[derive(Debug)]
pub struct PackReader<R> {
    source: R,
    entries: Vec<PackEntry>,
    by_id: HashMap<AssetId, usize>,
    by_path: HashMap<PackPath, usize>,
    payload_end: u64,
}

impl<R: ReadAt> PackReader<R> {
    /// Opens the pack in `source`: the header, then the index, both
    /// checked, and every entry checked to lie inside the payload.
    ///
    /// # Errors
    ///
    /// [`PackError`] for a file that is not a pack, is of a newer version,
    /// is damaged or truncated, or whose index is not valid. It never
    /// reads or allocates more than the file holds.
    pub fn open(source: R) -> Result<Self, PackError> {
        let file_len = source.size().map_err(|source| PackError::Read {
            offset: 0,
            length: 0,
            source,
        })?;
        let header = read_header(&source, file_len)?;
        let index = read_index(&source, &header, file_len)?;
        let entries = decode_index(&index, header.offset)?;
        let payload_end = header.offset;
        let (by_id, by_path) = check_entries(&entries, payload_end)?;
        Ok(PackReader {
            source,
            entries,
            by_id,
            by_path,
            payload_end,
        })
    }

    /// Every entry, in the order they were written.
    #[must_use]
    pub fn entries(&self) -> &[PackEntry] {
        &self.entries
    }

    /// The entry with this id.
    #[must_use]
    pub fn entry_by_id(&self, id: AssetId) -> Option<&PackEntry> {
        self.by_id.get(&id).map(|&i| &self.entries[i])
    }

    /// The entry with this path.
    #[must_use]
    pub fn entry_by_path(&self, path: &PackPath) -> Option<&PackEntry> {
        self.by_path.get(path).map(|&i| &self.entries[i])
    }

    /// The bytes of the asset `id`, read and checked.
    ///
    /// # Errors
    ///
    /// [`PackError::NoSuchId`], or the errors of [`read_entry`](Self::read_entry).
    pub fn read_id(&self, id: AssetId) -> Result<Vec<u8>, PackError> {
        let entry = self.entry_by_id(id).ok_or(PackError::NoSuchId(id))?;
        self.read_entry(entry)
    }

    /// The bytes of the asset at `path`, read and checked.
    ///
    /// # Errors
    ///
    /// [`PackError::NoSuchPath`], or the errors of [`read_entry`](Self::read_entry).
    pub fn read_path(&self, path: &PackPath) -> Result<Vec<u8>, PackError> {
        let entry = self
            .entry_by_path(path)
            .ok_or_else(|| PackError::NoSuchPath(path.clone()))?;
        self.read_entry(entry)
    }

    /// The bytes of `entry`: only its range is read, and the checksum is
    /// verified before they are returned.
    ///
    /// # Errors
    ///
    /// [`PackError::PayloadChecksum`] for damaged bytes,
    /// [`PackError::EntryOutOfBounds`] for an entry that is not this
    /// pack's, [`PackError::Read`] when the source fails.
    pub fn read_entry(&self, entry: &PackEntry) -> Result<Vec<u8>, PackError> {
        if !lies_in_payload(entry, self.payload_end) {
            return Err(PackError::EntryOutOfBounds {
                entry: self
                    .entries
                    .iter()
                    .position(|e| e == entry)
                    .unwrap_or(usize::MAX),
                path: entry.path.clone(),
                offset: entry.offset,
                len: entry.len,
                payload_start: HEADER_LEN as u64,
                payload_end: self.payload_end,
            });
        }
        let size = usize::try_from(entry.len).map_err(|_| PackError::TooLargeForMemory {
            path: entry.path.clone(),
            len: entry.len,
        })?;
        let mut bytes = vec![0_u8; size];
        read_exact(&self.source, entry.offset, &mut bytes)?;
        let computed = crc32(&bytes);
        if computed != entry.crc32 {
            return Err(PackError::PayloadChecksum {
                path: entry.path.clone(),
                offset: entry.offset,
                len: entry.len,
                stored: entry.crc32,
                computed,
            });
        }
        Ok(bytes)
    }

    /// Reads every asset and checks its checksum: what a build or an
    /// installer runs once, since [`open`](Self::open) checks only the
    /// header and the index.
    ///
    /// # Errors
    ///
    /// The first error of [`read_entry`](Self::read_entry).
    pub fn verify(&self) -> Result<(), PackError> {
        self.entries
            .iter()
            .try_for_each(|e| self.read_entry(e).map(drop))
    }
}
