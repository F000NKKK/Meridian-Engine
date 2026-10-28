//! [`EntrySet`]: the entries of a pack being built.

use std::collections::HashSet;

use super::index::IndexDocument;
use super::{PackBuildError, PackEntry, PackPath};
use crate::AssetId;

/// The entries added so far, in order, with the rule that ids and paths are
/// unique. Both writers (in memory and streaming) keep one, so they agree on
/// what a pack may hold and on the index they write.
#[derive(Debug, Default)]
pub(super) struct EntrySet {
    entries: Vec<PackEntry>,
    ids: HashSet<AssetId>,
    paths: HashSet<PackPath>,
}

impl EntrySet {
    /// Whether `id` and `path` are both still free.
    pub(super) fn check(&self, id: AssetId, path: &PackPath) -> Result<(), PackBuildError> {
        if self.ids.contains(&id) {
            return Err(PackBuildError::DuplicateId(id));
        }
        if self.paths.contains(path) {
            return Err(PackBuildError::DuplicatePath(path.clone()));
        }
        Ok(())
    }

    /// Adds `entry`.
    pub(super) fn push(&mut self, entry: PackEntry) -> Result<(), PackBuildError> {
        self.check(entry.id, &entry.path)?;
        self.ids.insert(entry.id);
        self.paths.insert(entry.path.clone());
        self.entries.push(entry);
        Ok(())
    }

    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    /// The index document, Substrate Data binary (version 2).
    pub(super) fn index_bytes(&self) -> Vec<u8> {
        substrate_data::to_bytes(&IndexDocument::of(&self.entries))
    }
}
