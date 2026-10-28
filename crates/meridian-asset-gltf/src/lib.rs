//! glTF 2.0 adapter for `meridian-asset-domain`: decodes `.glb` and self-contained `.gltf` static models on the `gltf` crate.
//!
//! Layer: infrastructure; bounded context: assets (`craftbook.toml`,
//! docs/context-map.md). [`GltfDecoder`] implements the `Decoder` port of
//! the domain (ADR 030).

#![forbid(unsafe_code)]

mod buffers;
mod data_uri;
mod gltf_decoder;
mod gltf_error;
mod limits;
mod read;
mod resources;
mod scene_walk;
#[cfg(test)]
mod skin_tests;
#[cfg(test)]
mod tests;

pub use gltf_decoder::GltfDecoder;
pub use gltf_error::GltfError;
pub use resources::Resources;
