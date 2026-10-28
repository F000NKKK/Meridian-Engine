//! [`StreamingPackWriter`]: writes a pack to a sink as assets arrive.

use super::PackWriteError;
use super::open_asset::OpenAsset;
use crate::AssetId;
use crate::pack::entry_set::EntrySet;
use crate::pack::layout::{Crc32, HEADER_LEN, IndexLocation, crc32};
use crate::pack::{PackEntry, PackPath, WriteAt};

/// Writes a pack through a [`WriteAt`] while the assets arrive, holding only
/// the entry list (a few dozen bytes per asset) and the one chunk in hand,
/// never the payloads. The bytes are exactly those of
/// [`PackWriter`](crate::PackWriter) for the same assets in the same order.
///
/// A pack puts its header first, but the header holds the index's place and
/// checksum, known only at the end. So the writer reserves 32 zero bytes
/// (not a valid header: an abandoned file is never read as a pack), writes
/// the payloads after them with each CRC-32 computed as the chunks go by,
/// writes the index after the last payload, and only then writes the header
/// over the reserved bytes. That is why the sink is positioned
/// ([`WriteAt`]) and not a plain forward-only stream.
///
/// One asset is written whole with [`add`](Self::add), or in chunks with
/// [`begin`](Self::begin), [`write`](Self::write) and [`end`](Self::end). An
/// error leaves the writer unusable; drop it and discard the sink.
#[derive(Debug)]
pub struct StreamingPackWriter<S> {
    sink: S,
    entries: EntrySet,
    position: u64,
    open: Option<OpenAsset>,
}

impl<S: WriteAt> StreamingPackWriter<S> {
    /// A writer at the start of `sink`, which must be empty.
    ///
    /// # Errors
    ///
    /// [`PackWriteError::Sink`] when the reserved header cannot be written.
    pub fn new(mut sink: S) -> Result<Self, PackWriteError> {
        sink.write_at(0, &[0_u8; HEADER_LEN])?;
        Ok(StreamingPackWriter {
            sink,
            entries: EntrySet::default(),
            position: HEADER_LEN as u64,
            open: None,
        })
    }

    /// Starts the asset `id` at `path`; its bytes follow with
    /// [`write`](Self::write).
    ///
    /// # Errors
    ///
    /// [`PackWriteError::Build`] when `id` or `path` is already in the pack,
    /// [`PackWriteError::AssetOpen`] when another asset is open.
    pub fn begin(&mut self, id: AssetId, path: PackPath) -> Result<(), PackWriteError> {
        if self.open.is_some() {
            return Err(PackWriteError::AssetOpen);
        }
        self.entries.check(id, &path)?;
        self.open = Some(OpenAsset {
            id,
            path,
            offset: self.position,
            len: 0,
            crc: Crc32::new(),
        });
        Ok(())
    }

    /// Appends `chunk` to the open asset.
    ///
    /// # Errors
    ///
    /// [`PackWriteError::NoAssetOpen`], [`PackWriteError::TooLarge`] or
    /// [`PackWriteError::Sink`].
    pub fn write(&mut self, chunk: &[u8]) -> Result<(), PackWriteError> {
        let open = self.open.as_mut().ok_or(PackWriteError::NoAssetOpen)?;
        let next = self
            .position
            .checked_add(chunk.len() as u64)
            .ok_or(PackWriteError::TooLarge)?;
        self.sink.write_at(self.position, chunk)?;
        open.crc.update(chunk);
        open.len += chunk.len() as u64;
        self.position = next;
        Ok(())
    }

    /// Ends the open asset and records it in the index.
    ///
    /// # Errors
    ///
    /// [`PackWriteError::NoAssetOpen`].
    pub fn end(&mut self) -> Result<(), PackWriteError> {
        let open = self.open.take().ok_or(PackWriteError::NoAssetOpen)?;
        self.entries.push(PackEntry {
            id: open.id,
            path: open.path,
            offset: open.offset,
            len: open.len,
            crc32: open.crc.finish(),
        })?;
        Ok(())
    }

    /// A whole asset in one call.
    ///
    /// # Errors
    ///
    /// As [`begin`](Self::begin), [`write`](Self::write) and [`end`](Self::end).
    pub fn add(&mut self, id: AssetId, path: PackPath, bytes: &[u8]) -> Result<(), PackWriteError> {
        self.begin(id, path)?;
        self.write(bytes)?;
        self.end()
    }

    /// How many assets are complete.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no asset is complete.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }

    /// Writes the index and then the header, and returns the sink.
    ///
    /// # Errors
    ///
    /// [`PackWriteError::AssetOpen`] when an asset was not ended,
    /// [`PackWriteError::TooLarge`] or [`PackWriteError::Sink`].
    pub fn finish(mut self) -> Result<S, PackWriteError> {
        if self.open.is_some() {
            return Err(PackWriteError::AssetOpen);
        }
        let index = self.entries.index_bytes();
        let location = IndexLocation {
            offset: self.position,
            len: index.len() as u64,
            crc32: crc32(&index),
        };
        self.position
            .checked_add(index.len() as u64)
            .ok_or(PackWriteError::TooLarge)?;
        self.sink.write_at(self.position, &index)?;
        self.sink.write_at(0, &location.encode())?;
        Ok(self.sink)
    }
}
