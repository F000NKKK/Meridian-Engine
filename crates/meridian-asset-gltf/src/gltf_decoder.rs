//! The glTF decoder.

use gltf::Gltf;
use meridian_asset_domain::{Decoder, ModelAsset};

use crate::limits::MAX_TOTAL_VERTICES;
use crate::{GltfError, buffers, image_reader, part_reader, scene_walk};

/// Decodes a `.glb`, or a `.gltf` whose buffers and images are `data:`
/// URIs, to world-space triangle parts (ADR 030). Skins, animations,
/// cameras, lights and extensions are ignored; a `.gltf` naming an external
/// file is [`GltfError::ExternalResource`].
#[derive(Debug, Default)]
pub struct GltfDecoder;

impl Decoder<ModelAsset> for GltfDecoder {
    type Error = GltfError;

    fn decode(&self, bytes: &[u8]) -> Result<ModelAsset, GltfError> {
        let Gltf { document, blob } =
            Gltf::from_slice(bytes).map_err(|err| GltfError::Malformed(err.to_string()))?;
        let buffers = buffers::load(&document, blob)?;
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
        let images = image_reader::read(&document, &buffers)?;
        Ok(ModelAsset { parts, images })
    }
}
