//! JPEG adapter for `meridian-asset-domain`: decodes JPEG bytes to RGBA8 on the `zune-jpeg` crate.
//!
//! Layer: infrastructure; bounded context: assets (`craftbook.toml`,
//! docs/context-map.md). [`JpegDecoder`] implements the `Decoder` port of
//! the domain, the way `meridian-asset-png` does for PNG (ADR 013, ADR 024).

#![forbid(unsafe_code)]

mod jpeg_decoder;
#[cfg(test)]
mod tests;

pub use jpeg_decoder::JpegDecoder;
