//! The byte layout of a pack: magic, version, the fixed header and the CRC-32.

mod checksum;
mod header;
mod index_location;
mod magic;
mod version;

pub(super) use checksum::crc32;
pub(super) use header::HEADER_LEN;
pub(super) use index_location::IndexLocation;
pub use magic::PACK_MAGIC;
pub use version::PACK_VERSION;
