//! HTTP adapter for `meridian-asset-app`'s [`ByteSource`](meridian_asset_app::ByteSource).
//!
//! Layer: infrastructure; bounded context: assets (`craftbook.toml`,
//! docs/context-map.md). Blocking reads over `http` and `https`: the loader runs
//! them on background threads (ADR 027, ADR 028).
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod asset_url;
mod http_source;
mod http_source_error;
#[cfg(test)]
mod test_server;
#[cfg(test)]
mod test_tls_server;
#[cfg(test)]
mod tests;

pub use http_source::HttpSource;
pub use http_source_error::HttpSourceError;
