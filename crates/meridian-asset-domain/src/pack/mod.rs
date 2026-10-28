//! The `.mpak` asset pack: payload blobs and an index, with random access by
//! asset id or path and a checksum on everything (docs/data-formats.md,
//! "Asset pack").

mod build_error;
mod entry;
mod entry_set;
mod error;
mod index;
mod layout;
mod path;
mod port;
mod read;
mod sink;
#[cfg(test)]
mod tests;
mod upgrade;
mod writer;

pub use build_error::PackBuildError;
pub use entry::PackEntry;
pub use error::PackError;
pub use layout::{PACK_MAGIC, PACK_VERSION};
pub use path::{PackPath, PackPathError};
pub use port::{ReadAt, ReadAtError, WriteAt, WriteAtError};
pub use read::{MAX_INDEX_LEN, PackReader};
pub use sink::{PackWriteError, StreamingPackWriter};
pub use upgrade::upgrade as upgrade_pack;
pub use writer::PackWriter;
