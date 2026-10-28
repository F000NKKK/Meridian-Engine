//! Rewriting a pack as the newest version.

use super::{PackError, PackReader, PackWriter, ReadAt};

/// A pack of any version this reads, as a pack of the newest one: what
/// reading an old file and saving it again does. The entries, their order,
/// ids, paths and every payload byte are kept; only the envelope and the
/// index (rewritten as Substrate Data binary version 2) are the newest.
/// Every payload is verified on the way, so a damaged pack is an error and
/// nothing is written.
///
/// # Errors
///
/// [`PackError`] when the pack does not open or an asset is damaged.
pub fn upgrade<R: ReadAt>(source: R) -> Result<Vec<u8>, PackError> {
    let reader = PackReader::open(source)?;
    let mut writer = PackWriter::new();
    for entry in reader.entries() {
        let bytes = reader.read_entry(entry)?;
        // Ids and paths were checked unique when the reader opened.
        writer.add(entry.id, entry.path.clone(), &bytes)?;
    }
    Ok(writer.finish())
}
