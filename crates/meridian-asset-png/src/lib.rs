//! PNG adapter for `meridian-asset-core`: decodes PNG bytes to RGBA8 on the `png` crate.
//!
//! Layer: infrastructure; bounded context: assets (`craftbook.toml`,
//! docs/context-map.md). [`PngDecoder`] implements the `Decoder` port of
//! the domain (ADR 013, ADR 024).

#![forbid(unsafe_code)]

mod png_decoder;

pub use png_decoder::PngDecoder;
