//! The `.mmat` reader and writer: round trips, defaults, formats, damage.

use super::*;
use crate::AssetKey;

fn gold() -> MaterialAsset {
    MaterialAsset {
        base_color: [1.0, 0.77, 0.34, 1.0],
        metallic: 1.0,
        roughness: 0.3,
        emissive: [0.1, 0.0, 0.0],
        blend: MaterialBlend::AlphaBlend,
        albedo: Some(AssetKey::new("textures/gold.png").unwrap()),
        normal: Some(AssetKey::new("textures/gold_n.png").unwrap()),
        ..MaterialAsset::default()
    }
}

#[test]
fn text_and_binary_read_back_the_same_material() {
    let material = gold();
    assert_eq!(
        MaterialAsset::read(material.to_text().as_bytes()),
        Ok(material.clone())
    );
    assert_eq!(MaterialAsset::read(&material.to_bytes()), Ok(material));
}

#[test]
fn the_text_names_textures_as_asset_references() {
    let text = gold().to_text();
    assert!(text.contains("@asset:textures/gold.png"), "{text}");
}

#[test]
fn omitted_fields_take_the_defaults() {
    let read = MaterialAsset::read(b"{ format: 1 }").unwrap();
    assert_eq!(read, MaterialAsset::default());
    let grey = MaterialAsset::read(b"{ format: 1, roughness: 0.9 }").unwrap();
    assert_eq!(
        (grey.roughness, grey.base_color),
        (0.9, [0.8, 0.8, 0.8, 1.0])
    );
}

#[test]
fn a_newer_format_is_unsupported_not_a_list_of_unknown_keys() {
    let err = MaterialAsset::read(b"{ format: 2, sheen: 1 }").unwrap_err();
    assert_eq!(err, MaterialError::UnsupportedFormat { found: 2 });
}

#[test]
fn a_missing_or_bad_format_is_named() {
    assert_eq!(
        MaterialAsset::read(b"{ roughness: 1 }"),
        Err(MaterialError::BadFormat)
    );
    assert_eq!(MaterialAsset::read(b"[1]"), Err(MaterialError::NotAMap));
}

#[test]
fn an_unknown_key_and_a_bad_blend_name_their_path() {
    let unknown = MaterialAsset::read(b"{ format: 1, sheen: 1 }").unwrap_err();
    assert!(unknown.to_string().contains("sheen"), "{unknown}");
    let blend = MaterialAsset::read(b"{ format: 1, blend: \"additive\" }").unwrap_err();
    assert!(blend.to_string().contains("blend"), "{blend}");
}

#[test]
fn a_texture_that_is_not_an_asset_reference_is_a_problem_at_its_field() {
    let err = MaterialAsset::read(b"{ format: 1, albedo: \"textures/a.png\" }").unwrap_err();
    assert!(err.to_string().contains("albedo"), "{err}");
}

#[test]
fn damaged_files_are_errors_never_panics() {
    let bytes = gold().to_bytes();
    for cut in 0..bytes.len() {
        drop(MaterialAsset::read(&bytes[..cut]));
    }
    for at in 0..bytes.len() {
        let mut flipped = bytes.clone();
        flipped[at] ^= 0xff;
        drop(MaterialAsset::read(&flipped));
    }
    assert!(MaterialAsset::read(&[0xff, 0xfe, 0x00]).is_err());
}

#[test]
fn the_textures_a_material_names_are_listed_in_a_fixed_order() {
    let material = gold();
    let names: Vec<&str> = material.textures().map(AssetKey::as_str).collect();
    assert_eq!(names, ["textures/gold.png", "textures/gold_n.png"]);
}
