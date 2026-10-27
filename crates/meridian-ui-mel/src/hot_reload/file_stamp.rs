//! [`FileStamp`]: what a file looked like when it was last read.

use std::path::Path;
use std::time::SystemTime;

/// Modification time and length; `None` stands for a file that is not
/// there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FileStamp {
    modified: Option<SystemTime>,
    len: u64,
}

impl FileStamp {
    pub(super) fn of(path: &Path) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        Some(Self {
            modified: meta.modified().ok(),
            len: meta.len(),
        })
    }
}
