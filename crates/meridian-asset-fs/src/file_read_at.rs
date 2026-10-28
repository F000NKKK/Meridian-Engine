//! A file as a [`ReadAt`].

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

use meridian_asset_domain::{ReadAt, ReadAtError};

/// A file opened for positioned reads, so a pack reads only the bytes it is
/// asked for. Reads take a lock for the seek and the read, so the source is
/// shareable between threads; they are short and each is one syscall pair.
#[derive(Debug)]
pub struct FileReadAt {
    file: Mutex<File>,
    size: u64,
}

impl FileReadAt {
    /// Opens `path` for reading.
    ///
    /// # Errors
    ///
    /// [`ReadAtError::Failed`] with the system's text when it cannot be
    /// opened or measured.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ReadAtError> {
        let failed = |error: std::io::Error| ReadAtError::Failed {
            reason: error.to_string(),
        };
        let file = File::open(path).map_err(failed)?;
        let size = file.metadata().map_err(failed)?.len();
        Ok(FileReadAt {
            file: Mutex::new(file),
            size,
        })
    }
}

impl ReadAt for FileReadAt {
    fn size(&self) -> Result<u64, ReadAtError> {
        Ok(self.size)
    }

    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ReadAtError> {
        let length = buf.len() as u64;
        let in_range = offset
            .checked_add(length)
            .is_some_and(|end| end <= self.size);
        if !in_range {
            return Err(ReadAtError::OutOfRange {
                offset,
                length,
                size: self.size,
            });
        }
        let failed = |error: std::io::Error| ReadAtError::Failed {
            reason: error.to_string(),
        };
        let mut file = self.file.lock().map_err(|_| ReadAtError::Failed {
            reason: "the file lock was poisoned".to_owned(),
        })?;
        file.seek(SeekFrom::Start(offset)).map_err(failed)?;
        file.read_exact(buf).map_err(failed)
    }
}
