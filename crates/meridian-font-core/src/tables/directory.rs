//! The table directory at the head of an sfnt file.

use crate::FontError;
use crate::reader::Reader;

const TRUE_TYPE: u32 = 0x0001_0000;
const TRUE: u32 = u32::from_be_bytes(*b"true");

#[derive(Debug)]
pub(crate) struct Directory<'a> {
    data: &'a [u8],
    records: Vec<Record>,
}

#[derive(Debug)]
struct Record {
    tag: [u8; 4],
    offset: usize,
    length: usize,
}

impl<'a> Directory<'a> {
    pub(crate) fn parse(data: &'a [u8]) -> Result<Self, FontError> {
        let mut reader = Reader::new(data, "table directory");
        match reader.u32()? {
            TRUE_TYPE | TRUE => {}
            _ => return Err(FontError::Unsupported("not a TrueType sfnt file")),
        }
        let count = reader.u16()?;
        reader.skip(6)?;
        let mut records = Vec::new();
        for _ in 0..count {
            let tag = reader
                .bytes(4)?
                .try_into()
                .map_err(|_| FontError::Malformed("table tag"))?;
            reader.skip(4)?;
            let offset = to_usize(reader.u32()?)?;
            let length = to_usize(reader.u32()?)?;
            records.push(Record {
                tag,
                offset,
                length,
            });
        }
        Ok(Self { data, records })
    }

    /// The bytes of table `tag`, or `Ok(None)` when the font has no such table.
    pub(crate) fn table(&self, tag: [u8; 4]) -> Result<Option<&'a [u8]>, FontError> {
        let Some(record) = self.records.iter().find(|record| record.tag == tag) else {
            return Ok(None);
        };
        let end = record.offset.checked_add(record.length);
        end.and_then(|end| self.data.get(record.offset..end))
            .map(Some)
            .ok_or(FontError::Truncated { what: "table data" })
    }

    pub(crate) fn required(&self, tag: [u8; 4], name: &'static str) -> Result<&'a [u8], FontError> {
        self.table(tag)?.ok_or(FontError::MissingTable(name))
    }
}

fn to_usize(value: u32) -> Result<usize, FontError> {
    usize::try_from(value).map_err(|_| FontError::Malformed("table offset"))
}
