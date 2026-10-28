//! Big-endian cursor over font bytes; every read is bounds-checked.

use crate::FontError;

#[derive(Debug, Clone)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    what: &'static str,
}

impl<'a> Reader<'a> {
    pub(crate) const fn new(data: &'a [u8], what: &'static str) -> Self {
        Self { data, pos: 0, what }
    }

    pub(crate) fn seek(&mut self, pos: usize) -> Result<(), FontError> {
        if pos > self.data.len() {
            return Err(self.truncated());
        }
        self.pos = pos;
        Ok(())
    }

    pub(crate) fn skip(&mut self, count: usize) -> Result<(), FontError> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or_else(|| self.truncated())?;
        self.seek(end)
    }

    pub(crate) const fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub(crate) fn bytes(&mut self, count: usize) -> Result<&'a [u8], FontError> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or_else(|| self.truncated())?;
        let slice = self
            .data
            .get(self.pos..end)
            .ok_or_else(|| self.truncated())?;
        self.pos = end;
        Ok(slice)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, FontError> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn i8(&mut self) -> Result<i8, FontError> {
        Ok(i8::from_be_bytes(self.array()?))
    }

    pub(crate) fn u16(&mut self) -> Result<u16, FontError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub(crate) fn i16(&mut self) -> Result<i16, FontError> {
        Ok(i16::from_be_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, FontError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], FontError> {
        let mut out = [0_u8; N];
        out.copy_from_slice(self.bytes(N)?);
        Ok(out)
    }

    const fn truncated(&self) -> FontError {
        FontError::Truncated { what: self.what }
    }
}
