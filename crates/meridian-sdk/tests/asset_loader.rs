//! The sdk's loader front door: files on disk to decoded images, off-thread.

use std::fs;

use meridian_sdk::{LoadError, SourceError, asset_loader, load_image};

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("meridian-sdk-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// A 1x1 24-bit BMP, red.
fn tiny_bmp() -> Vec<u8> {
    let mut bytes = b"BM".to_vec();
    bytes.extend_from_slice(&58_u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 4]);
    bytes.extend_from_slice(&54_u32.to_le_bytes());
    bytes.extend_from_slice(&40_u32.to_le_bytes());
    bytes.extend_from_slice(&1_i32.to_le_bytes());
    bytes.extend_from_slice(&1_i32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&24_u16.to_le_bytes());
    bytes.extend_from_slice(&[0; 24]);
    bytes.extend_from_slice(&[0, 0, 255, 0]);
    bytes
}

#[test]
fn an_image_loads_in_the_background() {
    let dir = scratch("image");
    fs::write(dir.join("red.bmp"), tiny_bmp()).expect("write image");
    let loader = asset_loader(&dir, 2);

    let image = load_image(&loader, "red.bmp").wait().expect("decoded");

    assert_eq!((image.width, image.height), (1, 1));
    fs::remove_dir_all(&dir).expect("remove scratch dir");
}

#[test]
fn a_missing_image_reports_not_found() {
    let dir = scratch("missing");
    let loader = asset_loader(&dir, 1);

    let result = load_image(&loader, "nope.png").wait();

    assert!(matches!(
        result,
        Err(LoadError::Source(SourceError::NotFound(_)))
    ));
    fs::remove_dir_all(&dir).expect("remove scratch dir");
}
