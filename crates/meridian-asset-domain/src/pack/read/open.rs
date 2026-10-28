//! The steps of opening a pack: header, index, entries.

use std::collections::HashMap;

use substrate_data::binary;

use super::pack_reader::MAX_INDEX_LEN;
use crate::AssetId;
use crate::pack::index::IndexDocument;
use crate::pack::layout::{HEADER_LEN, IndexLocation, crc32};
use crate::pack::{PackEntry, PackError, PackPath, ReadAt, ReadAtError};

pub(super) fn read_exact<R: ReadAt>(
    source: &R,
    offset: u64,
    buf: &mut [u8],
) -> Result<(), PackError> {
    source
        .read_exact_at(offset, buf)
        .map_err(|source: ReadAtError| PackError::Read {
            offset,
            length: buf.len() as u64,
            source,
        })
}

pub(super) fn read_header<R: ReadAt>(
    source: &R,
    file_len: u64,
) -> Result<IndexLocation, PackError> {
    if file_len < HEADER_LEN as u64 {
        return Err(PackError::TooShort {
            size: file_len,
            needed: HEADER_LEN as u64,
        });
    }
    let mut bytes = [0_u8; HEADER_LEN];
    read_exact(source, 0, &mut bytes)?;
    IndexLocation::decode(&bytes)
}

pub(super) fn read_index<R: ReadAt>(
    source: &R,
    header: &IndexLocation,
    file_len: u64,
) -> Result<Vec<u8>, PackError> {
    let layout_ok = header.offset >= HEADER_LEN as u64
        && header.offset.checked_add(header.len) == Some(file_len);
    if !layout_ok {
        return Err(PackError::BadLayout {
            index_offset: header.offset,
            index_len: header.len,
            file_len,
        });
    }
    if header.len > MAX_INDEX_LEN {
        return Err(PackError::IndexTooLarge {
            index_len: header.len,
            max: MAX_INDEX_LEN,
        });
    }
    // `index_len <= file_len` and `<= MAX_INDEX_LEN` here, so it fits.
    let mut index = vec![0_u8; usize::try_from(header.len).unwrap_or(0)];
    read_exact(source, header.offset, &mut index)?;
    let computed = crc32(&index);
    if computed != header.crc32 {
        return Err(PackError::IndexChecksum {
            offset: header.offset,
            stored: header.crc32,
            computed,
        });
    }
    Ok(index)
}

pub(super) fn decode_index(index: &[u8], index_offset: u64) -> Result<Vec<PackEntry>, PackError> {
    let value = binary::decode(index).map_err(|e| PackError::IndexMalformed {
        message: e.message,
        offset: index_offset + e.offset as u64,
    })?;
    let document = substrate_data::from_value::<IndexDocument>(&value)
        .map_err(PackError::IndexInvalid)?
        .value;
    Ok(document.entries.into_iter().map(PackEntry::from).collect())
}

pub(super) fn lies_in_payload(entry: &PackEntry, payload_end: u64) -> bool {
    entry.offset >= HEADER_LEN as u64
        && entry
            .offset
            .checked_add(entry.len)
            .is_some_and(|end| end <= payload_end)
}

pub(super) type Lookups = (HashMap<AssetId, usize>, HashMap<PackPath, usize>);

/// Checks every entry lies in the payload and no id or path repeats; the
/// lookup tables on the way.
pub(super) fn check_entries(entries: &[PackEntry], payload_end: u64) -> Result<Lookups, PackError> {
    let mut by_id = HashMap::with_capacity(entries.len());
    let mut by_path = HashMap::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        if !lies_in_payload(entry, payload_end) {
            return Err(PackError::EntryOutOfBounds {
                entry: index,
                path: entry.path.clone(),
                offset: entry.offset,
                len: entry.len,
                payload_start: HEADER_LEN as u64,
                payload_end,
            });
        }
        if let Some(first) = by_id.insert(entry.id, index) {
            return Err(PackError::DuplicateId {
                id: entry.id,
                first,
                second: index,
            });
        }
        if let Some(first) = by_path.insert(entry.path.clone(), index) {
            return Err(PackError::DuplicatePath {
                path: entry.path.clone(),
                first,
                second: index,
            });
        }
    }
    Ok((by_id, by_path))
}
