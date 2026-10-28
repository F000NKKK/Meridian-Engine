//! The streaming pack writer: a pack written through a [`WriteAt`](super::WriteAt)
//! without holding its payloads in memory.

mod open_asset;
mod pack_write_error;
mod streaming_pack_writer;

pub use pack_write_error::PackWriteError;
pub use streaming_pack_writer::StreamingPackWriter;
