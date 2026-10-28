//! [`WriteAt`]: the port a pack is written through.

use super::WriteAtError;

/// Positioned writes to one file, the streaming pack writer's only way out.
/// The domain does no I/O: a file is an adapter that implements this
/// (`meridian_asset_fs::FileWriteAt`). Positioned, because the header at the
/// start of a pack is written last, once the index is known.
pub trait WriteAt {
    /// Writes all of `bytes` at `offset`.
    ///
    /// # Errors
    ///
    /// [`WriteAtError::Failed`] when the system fails, and
    /// [`WriteAtError::Gap`] when the sink cannot have a hole.
    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), WriteAtError>;
}

impl WriteAt for Vec<u8> {
    /// Memory has no holes: `offset` is at most the current length.
    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), WriteAtError> {
        let gap = || WriteAtError::Gap {
            offset,
            length: bytes.len() as u64,
            size: self.len() as u64,
        };
        let start = usize::try_from(offset).map_err(|_| gap())?;
        if start > self.len() {
            return Err(gap());
        }
        let overlap = (self.len() - start).min(bytes.len());
        self[start..start + overlap].copy_from_slice(&bytes[..overlap]);
        self.extend_from_slice(&bytes[overlap..]);
        Ok(())
    }
}

impl<T: WriteAt + ?Sized> WriteAt for &mut T {
    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), WriteAtError> {
        (**self).write_at(offset, bytes)
    }
}
