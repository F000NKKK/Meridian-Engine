//! Tests of this module.

use super::*;

#[test]
fn obj_decodes_a_triangle() {
    let text = "# comment\nv 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3\n";
    let mesh = ObjDecoder.decode(text.as_bytes()).unwrap();
    assert_eq!(
        mesh.positions,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(mesh.indices, vec![0, 1, 2]);
}

#[test]
fn obj_decodes_texture_coordinates_and_normals() {
    let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvn 0 0 1\nf 1/1/1 2/2/1 3/3/1\n";
    let mesh = ObjDecoder.decode(text.as_bytes()).unwrap();
    assert_eq!(mesh.indices, vec![0, 1, 2]);
    assert_eq!(mesh.uvs, vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    assert_eq!(mesh.normals, vec![[0.0, 0.0, 1.0]; 3]);
}

#[test]
fn obj_splits_a_position_when_its_uv_changes() {
    let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 0 1\nvt 1 1\nf 1/1 2/2 3/3\nf 1/4 3/3 2/2\n";
    let mesh = ObjDecoder.decode(text.as_bytes()).unwrap();
    assert_eq!(mesh.positions.len(), 4);
    assert_eq!(mesh.indices, vec![0, 1, 2, 3, 2, 1]);
    assert_eq!(mesh.uvs[3], [1.0, 1.0]);
}

#[test]
fn obj_rejects_faces_with_more_than_three_indices() {
    let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 1 1 0\nf 1 2 3 4\n";
    assert_eq!(
        ObjDecoder.decode(text.as_bytes()),
        Err(DecodeError::Unsupported("non-triangular face"))
    );
}

#[test]
fn obj_rejects_faces_with_fewer_than_three_indices() {
    let text = "v 0 0 0\nv 1 0 0\nf 1 2\n";
    assert_eq!(
        ObjDecoder.decode(text.as_bytes()),
        Err(DecodeError::Unsupported("non-triangular face"))
    );
}

#[test]
fn shader_source_decodes_utf8_text_verbatim() {
    let text = "#version 450\nvoid main() {}\n";
    let shader = ShaderSourceDecoder.decode(text.as_bytes()).unwrap();
    assert_eq!(shader.source, text);
}

#[test]
fn shader_source_rejects_invalid_utf8() {
    let bytes = [0xff, 0xfe, 0xfd];
    assert!(matches!(
        ShaderSourceDecoder.decode(&bytes),
        Err(DecodeError::Malformed(_))
    ));
}
