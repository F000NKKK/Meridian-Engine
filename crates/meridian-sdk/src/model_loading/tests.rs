//! Tests of model registration.

use meridian_asset_domain::{EncodedImage, ModelAsset, ModelMaterial, ModelPart};
use meridian_rendering_app::testing::FakeTextureStore;
use meridian_rendering_domain::MeshRegistry;

use super::*;

fn triangle(material: ModelMaterial) -> ModelPart {
    ModelPart {
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![0, 1, 2],
        material,
        ..ModelPart::default()
    }
}

fn png_1x1() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&[255, 0, 0, 255]).unwrap();
    writer.finish().unwrap();
    bytes
}

fn textured(index: usize) -> ModelMaterial {
    ModelMaterial {
        base_color: [0.5, 0.5, 0.5, 1.0],
        base_color_image: Some(index),
        metallic: 0.25,
        roughness: 0.75,
        ..ModelMaterial::default()
    }
}

#[test]
fn missing_normals_and_uvs_are_filled_in() {
    let model = ModelAsset {
        parts: vec![triangle(ModelMaterial::default())],
        images: Vec::new(),
        ..ModelAsset::default()
    };
    let mut meshes = MeshRegistry::new();
    let pieces = register(model, &mut meshes, &mut FakeTextureStore::new())
        .unwrap()
        .pieces;
    let source = meshes.get(pieces[0].mesh).unwrap();
    assert_eq!(source.normals.len(), 3);
    assert_eq!(source.uvs, vec![[0.0, 0.0]; 3]);
    assert_eq!(pieces[0].material.albedo, None);
}

#[test]
fn parts_sharing_an_image_share_one_texture() {
    let model = ModelAsset {
        parts: vec![triangle(textured(0)), triangle(textured(0))],
        images: vec![EncodedImage { bytes: png_1x1() }],
        ..ModelAsset::default()
    };
    let pieces = register(
        model,
        &mut MeshRegistry::new(),
        &mut FakeTextureStore::new(),
    )
    .unwrap()
    .pieces;
    assert!(pieces[0].material.albedo.is_some());
    assert_eq!(pieces[0].material.albedo, pieces[1].material.albedo);
    assert_eq!(pieces[0].material.base_color_factor, [0.5, 0.5, 0.5, 1.0]);
    assert_eq!(
        (pieces[0].material.metallic, pieces[0].material.roughness),
        (0.25, 0.75)
    );
}

#[test]
fn a_metallic_roughness_map_is_its_own_linear_texture() {
    let material = ModelMaterial {
        metallic_roughness_image: Some(0),
        ..textured(0)
    };
    let model = ModelAsset {
        parts: vec![triangle(material), triangle(material)],
        images: vec![EncodedImage { bytes: png_1x1() }],
        ..ModelAsset::default()
    };
    let pieces = register(
        model,
        &mut MeshRegistry::new(),
        &mut FakeTextureStore::new(),
    )
    .unwrap()
    .pieces;
    let (first, second) = (pieces[0].material, pieces[1].material);
    assert!(first.metallic_roughness.is_some());
    assert_ne!(
        first.albedo, first.metallic_roughness,
        "one image, read as colour and as linear data, is two textures"
    );
    assert_eq!(first.metallic_roughness, second.metallic_roughness);
}

#[test]
fn normal_occlusion_and_emissive_maps_are_uploaded_by_how_they_are_read() {
    let material = ModelMaterial {
        normal_image: Some(0),
        normal_scale: 0.5,
        occlusion_image: Some(0),
        occlusion_strength: 0.25,
        emissive: [1.0, 0.5, 0.0],
        emissive_image: Some(0),
        ..ModelMaterial::default()
    };
    let model = ModelAsset {
        parts: vec![triangle(material)],
        images: vec![EncodedImage { bytes: png_1x1() }],
        ..ModelAsset::default()
    };
    let pieces = register(
        model,
        &mut MeshRegistry::new(),
        &mut FakeTextureStore::new(),
    )
    .unwrap()
    .pieces;
    let made = pieces[0].material;
    assert_eq!(made.normal, made.occlusion, "both are linear data");
    assert_ne!(made.normal, made.emissive_map, "emissive is colour data");
    assert_eq!(
        (made.normal_scale, made.occlusion_strength, made.emissive),
        (0.5, 0.25, [1.0, 0.5, 0.0])
    );
}

#[test]
fn an_undecodable_image_leaves_the_base_colour() {
    let model = ModelAsset {
        parts: vec![triangle(textured(0))],
        images: vec![EncodedImage {
            bytes: b"\xff\xd8\xff jpeg".to_vec(),
        }],
        ..ModelAsset::default()
    };
    let pieces = register(
        model,
        &mut MeshRegistry::new(),
        &mut FakeTextureStore::new(),
    )
    .unwrap()
    .pieces;
    assert_eq!(pieces[0].material.albedo, None);
}

#[test]
fn a_skinned_part_registers_its_skin_and_names_its_skeleton() {
    let part = ModelPart {
        skin: Some(meridian_asset_domain::PartSkin {
            skeleton: 2,
            joints: vec![[0, 1, 0, 0]; 3],
            weights: vec![[0.5, 0.5, 0.0, 0.0]; 3],
        }),
        ..triangle(ModelMaterial::default())
    };
    let model = ModelAsset {
        parts: vec![part],
        ..ModelAsset::default()
    };
    let mut meshes = MeshRegistry::new();
    let loaded = register(model, &mut meshes, &mut FakeTextureStore::new()).unwrap();
    assert_eq!(loaded.pieces[0].skeleton, Some(2));
    let skin = meshes.get(loaded.pieces[0].mesh).unwrap().skin.as_ref();
    assert_eq!(skin.map(|skin| skin.joints[0]), Some([0, 1, 0, 0]));
}
