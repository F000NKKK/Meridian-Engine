//! Adding a file to a streaming pack.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use meridian_asset_domain::{AssetId, PackPath, PackWriteError, StreamingPackWriter, WriteAt};
use thiserror::Error;

/// How much of the file is in memory at once.
const CHUNK: usize = 64 * 1024;

/// Why a file could not be added to a pack.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum AddFileError {
    /// The file could not be opened or read; `reason` is the system's text.
    #[error("cannot read {path}: {reason}")]
    Read {
        /// The file.
        path: String,
        /// What the system said.
        reason: String,
    },
    /// The pack refused it.
    #[error(transparent)]
    Pack(#[from] PackWriteError),
}

/// Copies the file at `file` into `writer` as the asset `id` at `path`, 64 KiB
/// at a time: the file is never held whole. Blocking. After an error the
/// writer is unusable (an asset may be half written): drop it and delete the
/// output.
///
/// # Errors
///
/// [`AddFileError::Read`] when the file does not open or read,
/// [`AddFileError::Pack`] when the pack refuses (a duplicate id or path, a
/// failing sink).
pub fn add_file<S: WriteAt>(
    writer: &mut StreamingPackWriter<S>,
    id: AssetId,
    path: PackPath,
    file: &Path,
) -> Result<(), AddFileError> {
    let read_error = |error: std::io::Error| AddFileError::Read {
        path: file.display().to_string(),
        reason: error.to_string(),
    };
    let mut source = File::open(file).map_err(read_error)?;
    writer.begin(id, path)?;
    let mut buffer = vec![0_u8; CHUNK];
    loop {
        let read = source.read(&mut buffer).map_err(read_error)?;
        if read == 0 {
            break;
        }
        writer.write(&buffer[..read])?;
    }
    writer.end()?;
    Ok(())
}
