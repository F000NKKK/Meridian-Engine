//! A scratch directory for the tests of this crate.

use std::fs;
use std::path::PathBuf;

/// A fresh empty directory under the system temp dir, removed on drop.
pub(crate) struct Scratch(pub(crate) PathBuf);

impl Scratch {
    /// `test` keeps the directories of parallel tests apart.
    pub(crate) fn new(test: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("meridian-asset-fs-{}-{test}", std::process::id()));
        fs::create_dir_all(&dir).expect("create scratch dir");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let removed = fs::remove_dir_all(&self.0);
        debug_assert!(removed.is_ok(), "scratch dir removal");
    }
}
