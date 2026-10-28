//! [`ReadAt`]: the port a pack is read through.

use super::ReadAtError;

/// Random access to the bytes of one file, the pack reader's only way in.
/// The domain does no I/O: a file, a memory map or a network range request
/// is an adapter that implements this. `&self` and positioned reads, so
/// one source serves several readers at once.
pub trait ReadAt {
    /// The size in bytes.
    ///
    /// # Errors
    ///
    /// [`ReadAtError::Failed`] when the source cannot say.
    fn size(&self) -> Result<u64, ReadAtError>;

    /// Fills `buf` with the bytes at `offset`.
    ///
    /// # Errors
    ///
    /// [`ReadAtError::OutOfRange`] when the range is not all inside the
    /// source (nothing is promised about `buf` then), and
    /// [`ReadAtError::Failed`] when the system fails.
    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ReadAtError>;
}

impl ReadAt for [u8] {
    fn size(&self) -> Result<u64, ReadAtError> {
        Ok(self.len() as u64)
    }

    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ReadAtError> {
        let out_of_range = || ReadAtError::OutOfRange {
            offset,
            length: buf.len() as u64,
            size: self.len() as u64,
        };
        let start = usize::try_from(offset).map_err(|_| out_of_range())?;
        let end = start.checked_add(buf.len()).ok_or_else(out_of_range)?;
        let source = self.get(start..end).ok_or_else(out_of_range)?;
        buf.copy_from_slice(source);
        Ok(())
    }
}

impl ReadAt for Vec<u8> {
    fn size(&self) -> Result<u64, ReadAtError> {
        self.as_slice().size()
    }

    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ReadAtError> {
        self.as_slice().read_exact_at(offset, buf)
    }
}

impl<T: ReadAt + ?Sized> ReadAt for &T {
    fn size(&self) -> Result<u64, ReadAtError> {
        (**self).size()
    }

    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ReadAtError> {
        (**self).read_exact_at(offset, buf)
    }
}
