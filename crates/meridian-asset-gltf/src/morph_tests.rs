//! Decoding morph targets and their weight animation (ADR 047).

use meridian_asset_domain::{Decoder, ModelAsset};

use crate::tests::glb;
use crate::{GltfDecoder, GltfError};

/// Positions, one target's offsets, key times, one weight per key and indices.
fn bin() -> Vec<u8> {
    let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let offsets: [f32; 9] = [0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 3.0];
    let keys: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
    let mut bin: Vec<u8> = positions
        .iter()
        .chain(&offsets)
        .chain(&keys)
        .flat_map(|f| f.to_le_bytes())
        .collect();
    for index in [0u16, 1, 2] {
        bin.extend(index.to_le_bytes());
    }
    bin
}

fn document(node_extra: &str, target_accessor: usize) -> String {
    format!(
        r#"{{
  "asset": {{"version": "2.0"}},
  "scene": 0,
  "scenes": [{{"nodes": [0]}}],
  "nodes": [{{"mesh": 0, "translation": [10, 0, 0]{node_extra}}}],
  "meshes": [{{"weights": [0.25], "primitives": [{{
    "attributes": {{"POSITION": 0}}, "indices": 4,
    "targets": [{{"POSITION": {target_accessor}}}]}}]}}],
  "animations": [{{
    "samplers": [{{"input": 2, "output": 3, "interpolation": "LINEAR"}}],
    "channels": [{{"sampler": 0, "target": {{"node": 0, "path": "weights"}}}}]
  }}],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
      "min": [0,0,0], "max": [1,1,0]}},
    {{"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3"}},
    {{"bufferView": 2, "componentType": 5126, "count": 2, "type": "SCALAR",
      "min": [0], "max": [1]}},
    {{"bufferView": 3, "componentType": 5126, "count": 2, "type": "SCALAR"}},
    {{"bufferView": 4, "componentType": 5123, "count": 3, "type": "SCALAR"}},
    {{"bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3"}}
  ],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 36, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 72, "byteLength": 8}},
    {{"buffer": 0, "byteOffset": 80, "byteLength": 8}},
    {{"buffer": 0, "byteOffset": 88, "byteLength": 6}}
  ],
  "buffers": [{{"byteLength": 94}}]
}}"#
    )
}

fn decode(node_extra: &str) -> Result<ModelAsset, GltfError> {
    GltfDecoder.decode(&glb(&document(node_extra, 1), &bin()))
}

#[test]
fn a_part_keeps_its_targets_placed_like_its_vertices() {
    let model = decode("").expect("a valid file");
    let morph = model.parts[0].morph.as_ref().expect("a morphed part");
    assert_eq!(morph.targets.len(), 1);
    // The node's translation moves vertices but not the offsets.
    assert_eq!(model.parts[0].positions[0], [10.0, 0.0, 0.0]);
    assert_eq!(morph.targets[0].positions[1], [0.0, 0.0, 2.0]);
    assert!(morph.targets[0].normals.is_empty());
}

#[test]
fn weights_start_from_the_mesh_and_a_node_overrides_them() {
    let from_mesh = decode("").expect("a valid file");
    assert_eq!(from_mesh.parts[0].morph.as_ref().unwrap().weights, [0.25]);
    let from_node = decode(r#", "weights": [0.75]"#).expect("a valid file");
    assert_eq!(from_node.parts[0].morph.as_ref().unwrap().weights, [0.75]);
}

#[test]
fn a_weights_animation_drives_the_part_its_node_made() {
    let model = decode("").expect("a valid file");
    let channel = &model.clips[0].weights[0];
    assert_eq!(channel.part, 0);
    assert_eq!(channel.times, [0.0, 1.0]);
    assert_eq!(channel.values, [0.0, 1.0]);
}

#[test]
fn a_target_of_the_wrong_size_is_refused() {
    let document = document("", 5);
    assert_eq!(
        GltfDecoder.decode(&glb(&document, &bin())),
        Err(GltfError::OutOfBounds("morph target size"))
    );
}
