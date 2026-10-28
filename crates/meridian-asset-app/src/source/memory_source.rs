//! An in-memory source for the tests of this crate.

use std::collections::BTreeMap;

use crate::{AssetListing, AssetPath, ByteSource, SourceError};

/// Files held in a map, listed in path order.
#[derive(Debug, Default)]
pub(crate) struct MemorySource(BTreeMap<String, Vec<u8>>);

impl MemorySource {
    pub(crate) fn with(mut self, path: &str, bytes: &[u8]) -> Self {
        self.0.insert(path.to_owned(), bytes.to_vec());
        self
    }
}

impl ByteSource for MemorySource {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        self.0
            .get(path.as_str())
            .cloned()
            .ok_or_else(|| SourceError::NotFound(path.clone()))
    }
}

impl AssetListing for MemorySource {
    fn paths(&self) -> Result<Vec<AssetPath>, SourceError> {
        Ok(self.0.keys().map(AssetPath::new).collect())
    }
}
