//! The glTF decoder.

use gltf::buffer::Source as BufferSource;
use gltf::image::Source as ImageSource;
use gltf::{Document, Gltf};
use meridian_asset_domain::{Decoder, ModelAsset};

use crate::limits::MAX_TOTAL_VERTICES;
use crate::resources::Resources;
use crate::{GltfError, buffers, image_reader, part_reader, scene_walk};

/// Decodes a `.glb`, or a `.gltf` whose buffers and images are `data:`
/// URIs or files supplied by the caller, to world-space triangle parts
/// (ADR 030). Skins, animations, cameras, lights and extensions are ignored.
/// [`Decoder::decode`] supplies no files, so a model naming one is
/// [`GltfError::ExternalResource`]; [`decode_with`](Self::decode_with) takes
/// them.
#[derive(Debug, Default)]
pub struct GltfDecoder;

impl GltfDecoder {
    /// The external files `bytes` names, in order of first use and without
    /// duplicates: what a loader must fetch before [`decode_with`](Self::decode_with).
    ///
    /// # Errors
    ///
    /// [`GltfError::Malformed`] when `bytes` is not a glTF document.
    pub fn required_uris(bytes: &[u8]) -> Result<Vec<String>, GltfError> {
        let Gltf { document, .. } = parse(bytes)?;
        let mut uris: Vec<String> = Vec::new();
        for uri in external_uris(&document) {
            if !uris.iter().any(|known| known == uri) {
                uris.push(uri.to_owned());
            }
        }
        Ok(uris)
    }

    /// Decodes `bytes`, resolving external URIs from `resources`.
    ///
    /// # Errors
    ///
    /// A [`GltfError`] when the file is malformed, names a file missing from
    /// `resources`, or exceeds a size bound.
    pub fn decode_with(bytes: &[u8], resources: &Resources) -> Result<ModelAsset, GltfError> {
        let Gltf { document, blob } = parse(bytes)?;
        let buffers = buffers::load(&document, blob, resources)?;
        let mut parts = Vec::new();
        let mut vertices = 0usize;
        for (node, world) in scene_walk::flatten(&document)? {
            let Some(mesh) = node.mesh() else { continue };
            for primitive in mesh.primitives() {
                let part = part_reader::read(&primitive, &world, &buffers)?;
                vertices = vertices.saturating_add(part.positions.len());
                if vertices > MAX_TOTAL_VERTICES {
                    return Err(GltfError::ModelTooLarge);
                }
                parts.push(part);
            }
        }
        let images = image_reader::read(&document, &buffers, resources)?;
        Ok(ModelAsset { parts, images })
    }
}

impl Decoder<ModelAsset> for GltfDecoder {
    type Error = GltfError;

    fn decode(&self, bytes: &[u8]) -> Result<ModelAsset, GltfError> {
        Self::decode_with(bytes, &Resources::new())
    }
}

fn parse(bytes: &[u8]) -> Result<Gltf, GltfError> {
    Gltf::from_slice(bytes).map_err(|err| GltfError::Malformed(err.to_string()))
}

/// Every buffer and image URI that is not a `data:` URI.
fn external_uris(document: &Document) -> impl Iterator<Item = &str> {
    let buffers = document
        .buffers()
        .filter_map(|buffer| match buffer.source() {
            BufferSource::Uri(uri) => Some(uri),
            BufferSource::Bin => None,
        });
    let images = document.images().filter_map(|image| match image.source() {
        ImageSource::Uri { uri, .. } => Some(uri),
        ImageSource::View { .. } => None,
    });
    buffers
        .chain(images)
        .filter(|uri| !uri.starts_with("data:"))
}
