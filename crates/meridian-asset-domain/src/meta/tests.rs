//! Tests of the `.meta` format.

use substrate_data::{Map, Value, binary};

use super::*;
use crate::AssetId;

fn id(bits: u64) -> AssetId {
    AssetId::from_bits(bits)
}

fn sample() -> AssetMeta {
    let mut meta = AssetMeta::new(id(0x3f2a_9c0e_8b1d_4a67), ImporterName::new("obj").unwrap());
    meta.settings.insert("scale", Value::Float(1.0));
    meta.settings.insert("generate_normals", Value::Bool(true));
    meta.dependencies = vec![id(1), id(2)];
    meta
}

#[test]
fn the_documented_example_reads() {
    let text = r#"{ format: 1, id: "3f2a9c0e8b1d4a67", importer: "obj",
        settings: { scale: 1.0, generate_normals: true }, // trailing comma ok
    }"#;
    let meta = AssetMeta::read(text.as_bytes()).unwrap();
    assert_eq!(meta.id.to_string(), "3f2a9c0e8b1d4a67");
    assert_eq!(meta.importer.as_str(), "obj");
    assert_eq!(meta.setting("generate_normals"), Some(&Value::Bool(true)));
    assert_eq!(meta.dependencies, []);
}

#[test]
fn text_and_binary_round_trip_to_the_same_meta() {
    let meta = sample();
    assert_eq!(AssetMeta::read(meta.to_text().as_bytes()).unwrap(), meta);
    assert_eq!(AssetMeta::read(&meta.to_bytes()).unwrap(), meta);
}

#[test]
fn settings_keep_file_order() {
    let read = AssetMeta::read(sample().to_text().as_bytes()).unwrap();
    let keys: Vec<&str> = read.settings.keys().collect();
    assert_eq!(keys, ["scale", "generate_normals"]);
}

#[test]
fn dependencies_are_typed_references() {
    let text = sample().to_text();
    assert!(text.contains("@asset_id:0000000000000001"), "{text}");
}

#[test]
fn writers_write_the_newest_format() {
    let text = sample().to_text();
    assert!(
        text.contains("format: 1") || text.contains("\"format\": 1"),
        "{text}"
    );
    let value = binary::decode(&sample().to_bytes()).unwrap();
    assert_eq!(
        value.as_map().unwrap().get("format"),
        Some(&Value::Int(i64::from(META_FORMAT)))
    );
}

#[test]
fn a_newer_format_is_unsupported_not_a_list_of_unknown_fields() {
    let text = r#"{ format: 2, id: "3f2a9c0e8b1d4a67", importer: "obj", brand_new: 1 }"#;
    assert_eq!(
        AssetMeta::read(text.as_bytes()),
        Err(MetaError::UnsupportedFormat { found: 2 })
    );
}

#[test]
fn a_missing_or_nonsense_format_is_bad_format() {
    for text in [
        r#"{ id: "3f2a9c0e8b1d4a67", importer: "obj" }"#,
        r#"{ format: 0, id: "3f2a9c0e8b1d4a67", importer: "obj" }"#,
        r#"{ format: -1, id: "3f2a9c0e8b1d4a67", importer: "obj" }"#,
        r#"{ format: "one", id: "3f2a9c0e8b1d4a67", importer: "obj" }"#,
    ] {
        assert_eq!(
            AssetMeta::read(text.as_bytes()),
            Err(MetaError::BadFormat),
            "{text}"
        );
    }
}

#[test]
fn a_root_that_is_not_a_map_is_refused() {
    assert_eq!(AssetMeta::read(b"[1, 2]"), Err(MetaError::NotAMap));
}

#[test]
fn problems_in_text_carry_path_line_and_column() {
    let text = "{\n  format: 1,\n  id: \"not-an-id\",\n  importer: \"obj\",\n}";
    let Err(MetaError::Invalid(problems)) = AssetMeta::read(text.as_bytes()) else {
        panic!("expected Invalid");
    };
    let shown = problems[0].to_string();
    assert!(shown.contains('3') && shown.contains("id"), "{shown}");
}

#[test]
fn unknown_missing_and_mistyped_fields_are_reported() {
    for text in [
        r#"{ format: 1, id: "3f2a9c0e8b1d4a67", importer: "obj", extra: 1 }"#,
        r#"{ format: 1, importer: "obj" }"#,
        r#"{ format: 1, id: "3f2a9c0e8b1d4a67" }"#,
        r#"{ format: 1, id: "3F2A9C0E8B1D4A67", importer: "obj" }"#,
        r#"{ format: 1, id: "3f2a9c0e8b1d4a6", importer: "obj" }"#,
        r#"{ format: 1, id: "3f2a9c0e8b1d4a67", importer: "Obj" }"#,
        r#"{ format: 1, id: "3f2a9c0e8b1d4a67", importer: "obj", dependencies: ["nope"] }"#,
    ] {
        assert!(
            matches!(AssetMeta::read(text.as_bytes()), Err(MetaError::Invalid(_))),
            "{text}"
        );
    }
}

#[test]
fn settings_must_be_a_map() {
    let text = r#"{ format: 1, id: "3f2a9c0e8b1d4a67", importer: "obj", settings: [1] }"#;
    assert_eq!(
        AssetMeta::read(text.as_bytes()),
        Err(MetaError::SettingsNotAMap)
    );
}

#[test]
fn a_dependency_on_itself_or_twice_is_refused() {
    let own = r#"{ format: 1, id: "0000000000000007", importer: "obj",
        dependencies: ["@asset_id:0000000000000007"] }"#;
    assert_eq!(
        AssetMeta::read(own.as_bytes()),
        Err(MetaError::SelfDependency { id: id(7) })
    );
    let twice = r#"{ format: 1, id: "0000000000000007", importer: "obj",
        dependencies: ["@asset_id:0000000000000001", "@asset_id:0000000000000002", "@asset_id:0000000000000001"] }"#;
    assert_eq!(
        AssetMeta::read(twice.as_bytes()),
        Err(MetaError::DuplicateDependency {
            id: id(1),
            first: 0,
            second: 2
        })
    );
}

#[test]
fn bytes_that_are_neither_text_nor_binary_are_typed_errors() {
    assert!(matches!(
        AssetMeta::read(&[0xff, 0xfe, 0x00]),
        Err(MetaError::NotUtf8 { valid_up_to: 0 })
    ));
    assert!(matches!(
        AssetMeta::read(b"SDAT"),
        Err(MetaError::Malformed { .. })
    ));
    assert!(matches!(AssetMeta::read(b""), Err(MetaError::Invalid(_))));
}

#[test]
fn truncated_and_flipped_files_never_panic() {
    let meta = sample();
    for file in [meta.to_bytes(), meta.to_text().into_bytes()] {
        for cut in 0..file.len() {
            drop(AssetMeta::read(&file[..cut]));
        }
        for at in 0..file.len() {
            for flip in [0x01_u8, 0x80, 0xff] {
                let mut damaged = file.clone();
                damaged[at] ^= flip;
                drop(AssetMeta::read(&damaged));
            }
        }
    }
}

#[test]
fn a_huge_declared_length_does_not_allocate_it() {
    // SDAT, version 2, then a list tag claiming u64::MAX / 2 items, no items.
    let mut bytes = b"SDAT\x02\x06".to_vec();
    bytes.extend_from_slice(&[0xff; 8]);
    bytes.push(0x7f);
    assert!(AssetMeta::read(&bytes).is_err());
}

#[test]
fn importer_names_are_checked() {
    assert!(ImporterName::new("gltf").is_ok());
    assert!(ImporterName::new("png-8bit_2").is_ok());
    for bad in ["", "Obj", "9obj", "o bj", "obj/x", &"a".repeat(65)] {
        assert!(ImporterName::new(bad).is_err(), "{bad}");
    }
}

#[test]
fn asset_ids_round_trip_through_text() {
    for bits in [0, 1, u64::MAX, 0x3f2a_9c0e_8b1d_4a67] {
        let id = AssetId::from_bits(bits);
        assert_eq!(id.to_string().parse::<AssetId>(), Ok(id));
        assert_eq!(id.to_string().len(), 16);
    }
    assert!("+f2a9c0e8b1d4a67".parse::<AssetId>().is_err());
}

#[test]
fn an_empty_settings_map_is_written_and_read() {
    let meta = AssetMeta::new(id(5), ImporterName::new("png").unwrap());
    let read = AssetMeta::read(meta.to_text().as_bytes()).unwrap();
    assert_eq!(read.settings, Map::new());
}
