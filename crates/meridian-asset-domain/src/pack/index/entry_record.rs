//! One entry of the pack index, as stored.

use substrate_data::Schema;

use crate::AssetId;
use crate::pack::{PackEntry, PackPath};

/// One entry, as stored.
#[derive(Schema)]
pub(in crate::pack) struct EntryRecord {
    id: AssetId,
    path: PackPath,
    offset: u64,
    len: u64,
    crc32: u32,
}

impl From<&PackEntry> for EntryRecord {
    fn from(entry: &PackEntry) -> Self {
        EntryRecord {
            id: entry.id,
            path: entry.path.clone(),
            offset: entry.offset,
            len: entry.len,
            crc32: entry.crc32,
        }
    }
}

impl From<EntryRecord> for PackEntry {
    fn from(record: EntryRecord) -> Self {
        PackEntry {
            id: record.id,
            path: record.path,
            offset: record.offset,
            len: record.len,
            crc32: record.crc32,
        }
    }
}
