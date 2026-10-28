//! The pack index as Substrate Data stores it (envelope version 1).

use substrate_data::Schema;

use super::entry_record::EntryRecord;
use crate::pack::PackEntry;

/// The index: a map with one key. Strict, so an index from a newer version
/// (more keys) is refused rather than half read; the envelope version is
/// what announces such a change.
#[derive(Schema)]
pub(in crate::pack) struct IndexDocument {
    pub(in crate::pack) entries: Vec<EntryRecord>,
}

impl IndexDocument {
    pub(in crate::pack) fn of(entries: &[PackEntry]) -> Self {
        IndexDocument {
            entries: entries.iter().map(EntryRecord::from).collect(),
        }
    }
}
