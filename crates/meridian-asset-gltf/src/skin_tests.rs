//! Decoding skins and joint animation (ADR 047).

use meridian_asset_domain::{Decoder, ModelAsset, ModelInterpolation, ModelProperty};

use crate::tests::glb;
use crate::{GltfDecoder, GltfError};

/// Three positions, `u8` joints, `f32` weights, `u16` indices, two inverse
/// bind matrices (the skin's joint order: node 1 first), animation times and translation values.
fn bin(joint_index: u8) -> Vec<u8> {
    let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let mut bin: Vec<u8> = positions.iter().flat_map(|f| f.to_le_bytes()).collect();
    for _ in 0..3 {
        bin.extend([0, joint_index, 0, 0]);
    }
    for _ in 0..3 {
        bin.extend([1.0f32, 1.0, 0.0, 0.0].iter().flat_map(|f| f.to_le_bytes()));
    }
    for index in [0u16, 1, 2, 0] {
        bin.extend(index.to_le_bytes());
    }
    let identity = [
        1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let lowered = [
        1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 1.0,
    ];
    bin.extend(
        lowered
            .iter()
            .chain(&identity)
            .flat_map(|f| f.to_le_bytes()),
    );
    let animation = [0.0f32, 1.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0];
    bin.extend(animation.iter().flat_map(|f| f.to_le_bytes()));
    bin
}

/// A skinned triangle on node 0 (moved by 100 on x, which a skin ignores)
/// bound to joints `[node 1, node 2]`, where node 1 is a child of node 2:
/// listed child first, so the decoder has to reorder.
fn document(mesh_attributes: &str) -> String {
    format!(
        r#"{{
  "asset": {{"version": "2.0"}},
  "scene": 0,
  "scenes": [{{"nodes": [0, 2]}}],
  "nodes": [
    {{"mesh": 0, "skin": 0, "translation": [100, 0, 0]}},
    {{"translation": [0, 1, 0]}},
    {{"children": [1]}}
  ],
  "skins": [{{"joints": [1, 2], "inverseBindMatrices": 4}}],
  "meshes": [{{"primitives": [{{"attributes": {{{mesh_attributes}}}, "indices": 3}}]}}],
  "animations": [{{
    "samplers": [{{"input": 5, "output": 6, "interpolation": "LINEAR"}}],
    "channels": [{{"sampler": 0, "target": {{"node": 1, "path": "translation"}}}}]
  }}],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
      "min": [0,0,0], "max": [1,1,0]}},
    {{"bufferView": 1, "componentType": 5121, "count": 3, "type": "VEC4"}},
    {{"bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC4"}},
    {{"bufferView": 3, "componentType": 5123, "count": 3, "type": "SCALAR"}},
    {{"bufferView": 4, "componentType": 5126, "count": 2, "type": "MAT4"}},
    {{"bufferView": 5, "componentType": 5126, "count": 2, "type": "SCALAR",
      "min": [0], "max": [1]}},
    {{"bufferView": 6, "componentType": 5126, "count": 2, "type": "VEC3"}}
  ],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 36, "byteLength": 12}},
    {{"buffer": 0, "byteOffset": 48, "byteLength": 48}},
    {{"buffer": 0, "byteOffset": 96, "byteLength": 6}},
    {{"buffer": 0, "byteOffset": 104, "byteLength": 128}},
    {{"buffer": 0, "byteOffset": 232, "byteLength": 8}},
    {{"buffer": 0, "byteOffset": 240, "byteLength": 24}}
  ],
  "buffers": [{{"byteLength": 264}}]
}}"#
    )
}

const SKINNED: &str = r#""POSITION": 0, "JOINTS_0": 1, "WEIGHTS_0": 2"#;

fn decode(attributes: &str, joint_index: u8) -> Result<ModelAsset, GltfError> {
    GltfDecoder.decode(&glb(&document(attributes), &bin(joint_index)))
}

#[test]
fn joints_are_ordered_parents_first_with_their_rest_pose_and_bind() {
    let model = decode(SKINNED, 0).expect("a valid skinned file");
    let skeleton = &model.skeletons[0];
    assert_eq!(skeleton.joints.len(), 2);
    assert_eq!(skeleton.joints[0].parent, None);
    assert_eq!(skeleton.joints[1].parent, Some(0));
    assert_eq!(skeleton.joints[1].translation, [0.0, 1.0, 0.0]);
    assert_eq!(skeleton.joints[1].inverse_bind[3], [0.0, -1.0, 0.0, 1.0]);
    assert_eq!(skeleton.root[3], [0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn a_skinned_part_ignores_its_node_and_follows_remapped_normalised_joints() {
    let model = decode(SKINNED, 1).expect("a valid skinned file");
    let part = &model.parts[0];
    assert_eq!(part.positions[1], [1.0, 0.0, 0.0]);
    let skin = part.skin.as_ref().expect("the part is skinned");
    assert_eq!(skin.skeleton, 0);
    // The file's joint 1 (node 2) is the skeleton's joint 0; joint 0 is 1.
    assert_eq!(skin.joints[0], [1, 0, 1, 1]);
    assert_eq!(skin.weights[0], [0.5, 0.5, 0.0, 0.0]);
}

#[test]
fn a_joint_animation_targets_the_reordered_joint() {
    let model = decode(SKINNED, 0).expect("a valid skinned file");
    let channel = &model.clips[0].channels[0];
    assert_eq!((channel.skeleton, channel.joint), (0, 1));
    assert_eq!(channel.property, ModelProperty::Translation);
    assert_eq!(channel.interpolation, ModelInterpolation::Linear);
    assert_eq!(channel.times, [0.0, 1.0]);
    assert_eq!(channel.values, [0.0, 1.0, 0.0, 0.0, 2.0, 0.0]);
}

#[test]
fn a_joint_index_past_the_skin_is_refused() {
    assert_eq!(
        decode(SKINNED, 2),
        Err(GltfError::OutOfBounds("joint index"))
    );
}

#[test]
fn a_primitive_without_joints_is_placed_like_a_rigid_part() {
    let model = decode(r#""POSITION": 0"#, 0).expect("a valid file");
    assert!(model.parts[0].skin.is_none());
    assert_eq!(model.parts[0].positions[0], [100.0, 0.0, 0.0]);
}
