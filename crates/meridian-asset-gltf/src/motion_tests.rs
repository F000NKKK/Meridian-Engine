//! Decoding animation of nodes that carry plain meshes (ADR 047).

use meridian_asset_domain::{Decoder, ModelAsset};

use crate::GltfDecoder;
use crate::tests::glb;

/// The skin fixture with the skin dropped and the animation on the mesh
/// node (`target`) instead of a joint.
fn decode(target: usize) -> ModelAsset {
    let document = super::skin_tests::document(r#""POSITION": 0"#)
        .replace(r#""skin": 0, "#, "")
        .replace(
            r#""target": {"node": 1"#,
            &format!(r#""target": {{"node": {target}"#),
        );
    GltfDecoder
        .decode(&glb(&document, &super::skin_tests::bin(0)))
        .expect("a valid file")
}

#[test]
fn an_animated_mesh_node_becomes_a_joint_its_part_is_bound_to() {
    let model = decode(0);
    // Skeleton 0 is the file's unused skin; the nodes' own comes after.
    let skeleton = &model.skeletons[1];
    assert_eq!(skeleton.joints.len(), 1);
    assert_eq!(skeleton.joints[0].translation, [100.0, 0.0, 0.0]);
    let skin = model.parts[0].skin.as_ref().expect("the part is posed");
    assert_eq!(skin.skeleton, 1);
    assert_eq!(skin.joints[0], [0, 0, 0, 0]);
    assert_eq!(skin.weights[0], [1.0, 0.0, 0.0, 0.0]);
    // In node space: the node's own placement is the joint's, not the vertices'.
    assert_eq!(model.parts[0].positions[0], [0.0, 0.0, 0.0]);
    let channel = &model.clips[0].channels[0];
    assert_eq!((channel.skeleton, channel.joint), (1, 0));
}

#[test]
fn a_mesh_no_animation_reaches_stays_rigid() {
    let model = decode(2);
    // Node 2 (animated) is not an ancestor of node 0: nothing moves the mesh.
    assert!(model.parts[0].skin.is_none());
}
