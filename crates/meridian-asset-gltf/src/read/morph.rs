//! Morph targets of a primitive and the weights animation drives.

use gltf::Primitive;
use meridian_asset_domain::{ModelPart, MorphTarget, PartMorph};

use crate::GltfError;
use crate::limits::MAX_MORPH_TARGETS;

/// The primitive's morph targets, all weighted zero, `None`
/// when it has none. Offsets are as authored, in the primitive's own space.
pub(crate) fn read(
    primitive: &Primitive<'_>,
    buffers: &[Vec<u8>],
    vertices: usize,
) -> Result<Option<PartMorph>, GltfError> {
    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
    let mut targets = Vec::new();
    for (positions, normals, _) in reader.read_morph_targets() {
        if targets.len() == MAX_MORPH_TARGETS {
            return Err(GltfError::OutOfBounds("morph targets"));
        }
        let positions: Vec<[f32; 3]> =
            positions.map_or_else(|| vec![[0.0; 3]; vertices], Iterator::collect);
        let normals: Vec<[f32; 3]> = normals.map(Iterator::collect).unwrap_or_default();
        if positions.len() != vertices || !(normals.is_empty() || normals.len() == vertices) {
            return Err(GltfError::OutOfBounds("morph target size"));
        }
        targets.push(MorphTarget { positions, normals });
    }
    if targets.is_empty() {
        return Ok(None);
    }
    let weights = vec![0.0; targets.len()];
    Ok(Some(PartMorph { targets, weights }))
}

/// Starts `part`'s weights from `weights`, the node's own or else the mesh's.
pub(crate) fn override_weights(part: &mut ModelPart, weights: Option<&[f32]>) {
    let (Some(morph), Some(weights)) = (&mut part.morph, weights) else {
        return;
    };
    for (slot, weight) in morph.weights.iter_mut().zip(weights) {
        *slot = *weight;
    }
}
