//! Decoding hand-built GLB and glTF files.

use meridian_asset_domain::{Decoder, ModelAsset};

use crate::{GltfDecoder, GltfError};

/// Positions, normals, UVs (f32) then `u16` indices of one triangle.
fn triangle_bin() -> Vec<u8> {
    let floats: [f32; 24] = [
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, // positions
        0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, // normals
        0.0, 0.0, 1.0, 0.0, 0.0, 1.0, // uvs
    ];
    let mut bin: Vec<u8> = floats.iter().flat_map(|f| f.to_le_bytes()).collect();
    for index in [0u16, 1, 2, 0] {
        bin.extend(index.to_le_bytes());
    }
    bin
}

pub(crate) fn glb(json: &str, bin: &[u8]) -> Vec<u8> {
    fn chunk(kind: [u8; 4], data: &[u8], pad: u8) -> Vec<u8> {
        let mut data = data.to_vec();
        while !data.len().is_multiple_of(4) {
            data.push(pad);
        }
        let mut out = u32::try_from(data.len()).unwrap().to_le_bytes().to_vec();
        out.extend(kind);
        out.extend(data);
        out
    }
    let mut body = chunk(*b"JSON", json.as_bytes(), b' ');
    body.extend(chunk(*b"BIN\0", bin, 0));
    let mut out = b"glTF".to_vec();
    out.extend(2u32.to_le_bytes());
    out.extend(u32::try_from(12 + body.len()).unwrap().to_le_bytes());
    out.extend(body);
    out
}

/// A one-triangle document; `node` is the JSON of the single node,
/// `primitive_extra` extra primitive members, `extra` top-level members.
fn document(buffer_uri: &str, node: &str, primitive_extra: &str, extra: &str) -> String {
    format!(
        r#"{{
  "asset": {{"version": "2.0"}},
  "scene": 0,
  "scenes": [{{"nodes": [0]}}],
  "nodes": [{{{node}"mesh": 0}}],
  "meshes": [{{"primitives": [{{
    "attributes": {{"POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2}},
    "indices": 3{primitive_extra}
  }}]}}],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
      "min": [0,0,0], "max": [1,1,0]}},
    {{"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3"}},
    {{"bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2"}},
    {{"bufferView": 3, "componentType": 5123, "count": 3, "type": "SCALAR"}}
  ],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 36, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 72, "byteLength": 24}},
    {{"buffer": 0, "byteOffset": 96, "byteLength": 6}}
  ],
  "buffers": [{{"byteLength": 104{buffer_uri}}}]{extra}
}}"#
    )
}

fn decode(node: &str, primitive_extra: &str) -> Result<ModelAsset, GltfError> {
    let json = document("", node, primitive_extra, "");
    GltfDecoder.decode(&glb(&json, &triangle_bin()))
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5)
}

#[test]
fn a_glb_triangle_keeps_its_attributes_and_default_material() {
    let model = decode("", "").unwrap();
    let part = &model.parts[0];
    assert_eq!(
        part.positions,
        [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
    );
    assert_eq!(part.normals, [[0.0, 0.0, 1.0]; 3]);
    assert_eq!(part.uvs, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    assert_eq!(part.indices, [0, 1, 2]);
    assert_eq!(part.material.base_color, [1.0; 4]);
    assert_eq!(part.material.base_color_image, None);
    // glTF's defaults: a fully metallic, fully rough surface.
    assert_eq!(
        (part.material.metallic, part.material.roughness),
        (1.0, 1.0)
    );
    assert!(model.images.is_empty());
}

#[test]
fn a_node_transform_moves_and_scales_the_geometry() {
    let model = decode(r#""translation": [0, 0, 5], "scale": [2, 2, 2], "#, "").unwrap();
    let part = &model.parts[0];
    assert!(close(part.positions[1], [2.0, 0.0, 5.0]));
    assert!(close(part.normals[0], [0.0, 0.0, 1.0]));
    assert_eq!(part.indices, [0, 1, 2]);
}

#[test]
fn a_mirroring_node_reverses_the_winding() {
    let model = decode(r#""scale": [-1, 1, 1], "#, "").unwrap();
    assert_eq!(model.parts[0].indices, [0, 2, 1]);
}

#[test]
fn normals_follow_the_inverse_transpose_under_nonuniform_scale() {
    let tilted = r#""scale": [2, 1, 1], "#;
    let model = decode(tilted, "").unwrap();
    // The surface normal +Z stays +Z; a unit normal stays unit length.
    let normal = model.parts[0].normals[0];
    let length = normal.iter().map(|v| v * v).sum::<f32>().sqrt();
    assert!((length - 1.0).abs() < 1e-5);
    assert!(close(normal, [0.0, 0.0, 1.0]));
}

#[test]
fn a_material_gives_its_base_colour_factors_and_texture() {
    let primitive_material = r#","material": 0"#;
    let extra = r#",
  "materials": [{"pbrMetallicRoughness": {
    "baseColorFactor": [0.5, 0.25, 1, 0.75],
    "metallicFactor": 0.25, "roughnessFactor": 0.75,
    "metallicRoughnessTexture": {"index": 0},
    "baseColorTexture": {"index": 0}}}],
  "textures": [{"source": 0}],
  "images": [{"bufferView": 4, "mimeType": "image/png"}]"#;
    let json = document("", "", primitive_material, extra).replace(
        r#"{"buffer": 0, "byteOffset": 96, "byteLength": 6}"#,
        r#"{"buffer": 0, "byteOffset": 96, "byteLength": 6},
    {"buffer": 0, "byteOffset": 96, "byteLength": 4}"#,
    );
    let model = GltfDecoder.decode(&glb(&json, &triangle_bin())).unwrap();
    let material = model.parts[0].material;
    assert_eq!(material.base_color, [0.5, 0.25, 1.0, 0.75]);
    assert_eq!(material.base_color_image, Some(0));
    assert_eq!((material.metallic, material.roughness), (0.25, 0.75));
    assert_eq!(material.metallic_roughness_image, Some(0));
    assert_eq!(model.images[0].bytes.len(), 4);
}

#[test]
fn a_material_gives_its_normal_occlusion_and_emissive_slots() {
    let extra = r#",
  "materials": [{
    "normalTexture": {"index": 0, "scale": 0.5},
    "occlusionTexture": {"index": 0, "strength": 0.25},
    "emissiveTexture": {"index": 0},
    "emissiveFactor": [1, 0.5, 0]}],
  "textures": [{"source": 0}],
  "images": [{"bufferView": 4, "mimeType": "image/png"}]"#;
    let json = document("", "", r#","material": 0"#, extra).replace(
        r#"{"buffer": 0, "byteOffset": 96, "byteLength": 6}"#,
        r#"{"buffer": 0, "byteOffset": 96, "byteLength": 6},
    {"buffer": 0, "byteOffset": 96, "byteLength": 4}"#,
    );
    let model = GltfDecoder.decode(&glb(&json, &triangle_bin())).unwrap();
    let material = model.parts[0].material;
    assert_eq!(
        (material.normal_image, material.normal_scale),
        (Some(0), 0.5)
    );
    assert_eq!(
        (material.occlusion_image, material.occlusion_strength),
        (Some(0), 0.25)
    );
    assert_eq!(
        (material.emissive_image, material.emissive),
        (Some(0), [1.0, 0.5, 0.0])
    );
}

#[test]
fn a_gltf_with_data_uris_decodes_without_a_second_file() {
    let bin = triangle_bin();
    let encoded = base64_encode(&bin);
    let uri = format!(r#", "uri": "data:application/octet-stream;base64,{encoded}""#);
    let json = document(&uri, "", "", "");
    let model = GltfDecoder.decode(json.as_bytes()).unwrap();
    assert_eq!(model.parts[0].indices, [0, 1, 2]);
}

#[test]
fn an_external_buffer_is_named_not_loaded() {
    let json = document(r#", "uri": "scene.bin""#, "", "", "");
    let error = GltfDecoder.decode(json.as_bytes()).unwrap_err();
    assert_eq!(error, GltfError::ExternalResource("scene.bin".to_owned()));
}

#[test]
fn a_bad_data_uri_is_rejected() {
    let json = document(r#", "uri": "data:text/plain,hello""#, "", "", "");
    assert_eq!(
        GltfDecoder.decode(json.as_bytes()),
        Err(GltfError::BadDataUri)
    );
    let garbage = document(r#", "uri": "data:;base64,@@@@""#, "", "", "");
    assert_eq!(
        GltfDecoder.decode(garbage.as_bytes()),
        Err(GltfError::BadDataUri)
    );
}

#[test]
fn only_triangle_lists_are_supported() {
    let error = decode("", r#", "mode": 5"#).unwrap_err();
    assert_eq!(
        error,
        GltfError::UnsupportedPrimitive("TriangleStrip".to_owned())
    );
}

#[test]
fn a_primitive_without_positions_is_rejected() {
    let json = document("", "", "", "").replace(r#""POSITION": 0, "#, "");
    let error = GltfDecoder
        .decode(&glb(&json, &triangle_bin()))
        .unwrap_err();
    assert!(matches!(error, GltfError::Malformed(_)));
}

#[test]
fn an_index_past_the_vertices_is_rejected() {
    let mut bin = triangle_bin();
    bin[96] = 9;
    let json = document("", "", "", "");
    let error = GltfDecoder.decode(&glb(&json, &bin)).unwrap_err();
    assert_eq!(error, GltfError::BadIndices);
}

#[test]
fn accessors_longer_than_their_buffer_are_rejected() {
    let json = document("", "", "", "");
    let short = &triangle_bin()[..60];
    assert!(GltfDecoder.decode(&glb(&json, short)).is_err());
}

#[test]
fn bytes_that_are_not_a_model_are_malformed() {
    assert!(matches!(
        GltfDecoder.decode(b"not a model"),
        Err(GltfError::Malformed(_))
    ));
    assert!(matches!(
        GltfDecoder.decode(b""),
        Err(GltfError::Malformed(_))
    ));
}

#[test]
fn a_file_without_scenes_uses_its_parentless_nodes() {
    let json = document("", "", "", "").replace(
        r#"  "scene": 0,
  "scenes": [{"nodes": [0]}],
"#,
        "",
    );
    let model = GltfDecoder.decode(&glb(&json, &triangle_bin())).unwrap();
    assert_eq!(model.parts.len(), 1);
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, b)| acc | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..=chunk.len() {
            out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
        for _ in chunk.len()..3 {
            out.push('=');
        }
    }
    out
}

#[test]
fn required_uris_lists_each_external_file_once() {
    let json = document(r#", "uri": "scene.bin""#, "", "", "");
    let uris = GltfDecoder::required_uris(json.as_bytes()).unwrap();
    assert_eq!(uris, ["scene.bin"]);
}

#[test]
fn data_uris_and_glb_chunks_are_not_required_files() {
    let encoded = base64_encode(&triangle_bin());
    let uri = format!(r#", "uri": "data:application/octet-stream;base64,{encoded}""#);
    let json = document(&uri, "", "", "");
    assert!(
        GltfDecoder::required_uris(json.as_bytes())
            .unwrap()
            .is_empty()
    );
    let glb = glb(&document("", "", "", ""), &triangle_bin());
    assert!(GltfDecoder::required_uris(&glb).unwrap().is_empty());
}

#[test]
fn decode_with_reads_a_supplied_external_buffer() {
    let json = document(r#", "uri": "scene.bin""#, "", "", "");
    let mut resources = crate::Resources::new();
    resources.insert("scene.bin".to_owned(), triangle_bin());
    let model = GltfDecoder::decode_with(json.as_bytes(), &resources).unwrap();
    assert_eq!(model.parts[0].indices, [0, 1, 2]);
}

#[test]
fn decode_with_still_names_a_file_it_was_not_given() {
    let json = document(r#", "uri": "scene.bin""#, "", "", "");
    let mut resources = crate::Resources::new();
    resources.insert("other.bin".to_owned(), triangle_bin());
    assert_eq!(
        GltfDecoder::decode_with(json.as_bytes(), &resources),
        Err(GltfError::ExternalResource("scene.bin".to_owned()))
    );
}

#[test]
fn an_external_image_is_read_from_the_supplied_files() {
    let extra = r#",
  "materials": [{"pbrMetallicRoughness": {"baseColorTexture": {"index": 0}}}],
  "textures": [{"source": 0}],
  "images": [{"uri": "tex/wall.png"}]"#;
    let json = document("", "", r#","material": 0"#, extra);
    let mut resources = crate::Resources::new();
    resources.insert("tex/wall.png".to_owned(), vec![1, 2, 3]);
    assert_eq!(
        GltfDecoder::required_uris(glb(&json, &triangle_bin()).as_slice()).unwrap(),
        ["tex/wall.png"]
    );
    let model = GltfDecoder::decode_with(&glb(&json, &triangle_bin()), &resources).unwrap();
    assert_eq!(model.images[0].bytes, [1, 2, 3]);
}
