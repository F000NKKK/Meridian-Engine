//! A file as a [`WriteAt`].

use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

use meridian_asset_domain::{WriteAt, WriteAtError};

/// A file created for positioned writes, so a pack can be written in one
/// pass and its header filled in last. Sequential writes do not seek.
#[derive(Debug)]
pub struct FileWriteAt {
    file: File,
    cursor: u64,
}

impl FileWriteAt {
    /// Creates `path` (or empties it) for writing.
    ///
    /// # Errors
    ///
    /// [`WriteAtError::Failed`] with the system's text when it cannot be
    /// created.
    pub fn create(path: impl AsRef<Path>) -> Result<Self, WriteAtError> {
        let file = File::create(path).map_err(|error| failed(&error))?;
        Ok(FileWriteAt { file, cursor: 0 })
    }

    /// Asks the system to put everything written on the disk.
    ///
    /// # Errors
    ///
    /// [`WriteAtError::Failed`] when the system says it could not.
    pub fn sync(&self) -> Result<(), WriteAtError> {
        self.file.sync_all().map_err(|error| failed(&error))
    }
}

impl WriteAt for FileWriteAt {
    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), WriteAtError> {
        if offset != self.cursor {
            self.file
                .seek(SeekFrom::Start(offset))
                .map_err(|error| failed(&error))?;
            self.cursor = offset;
        }
        // The file position after a failed write is unknown: force a seek next time.
        let written = self.file.write_all(bytes);
        self.cursor = if written.is_ok() {
            offset + bytes.len() as u64
        } else {
            u64::MAX
        };
        written.map_err(|error| failed(&error))
    }
}

fn failed(error: &std::io::Error) -> WriteAtError {
    WriteAtError::Failed {
        reason: error.to_string(),
    }
}
