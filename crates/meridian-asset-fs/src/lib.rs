//! File-system adapter for `meridian-asset-app`'s [`ByteSource`](meridian_asset_app::ByteSource).
//!
//! Layer: infrastructure; bounded context: assets (`craftbook.toml`,
//! docs/context-map.md). Blocking `std::fs` reads: the loader runs them on
//! background threads (ADR 027).
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod add_file;
mod file_read_at;
mod file_write_at;
mod fs_source;
#[cfg(test)]
mod pack_write_tests;
#[cfg(test)]
mod scratch;
#[cfg(test)]
mod tests;
mod walk;

pub use add_file::{AddFileError, add_file};
pub use file_read_at::FileReadAt;
pub use file_write_at::FileWriteAt;
pub use fs_source::FsSource;
