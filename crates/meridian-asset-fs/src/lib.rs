//! File-system adapter for `meridian-asset-app`'s [`ByteSource`](meridian_asset_app::ByteSource).
//!
//! Layer: infrastructure; bounded context: assets (`craftbook.toml`,
//! docs/context-map.md). Blocking `std::fs` reads: the loader runs them on
//! background threads (ADR 027).
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod file_read_at;
mod fs_source;
#[cfg(test)]
mod tests;

pub use file_read_at::FileReadAt;
pub use fs_source::FsSource;
