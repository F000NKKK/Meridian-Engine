//! A `.meta` file's `importer` name to decoded content and a runtime identity.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use meridian_sdk::content::{
    AssetCatalog, ImportError, Importers, ResourceId, ResourceIdTable, ResourcePool,
};
use meridian_sdk::{AssetLoader, AssetPath, BackgroundPool, FsSource, ImageAsset, LoadError};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("meridian-sdk-ci-{name}-{}", std::process::id()));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

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

fn write_meta(dir: &Path, asset: &str, id: &str, importer: &str) {
    let text = format!(r#"{{ format: 1, id: "{id}", importer: "{importer}" }}"#);
    fs::write(dir.join(format!("{asset}.meta")), text).expect("write meta");
}

#[test]
fn meta_files_give_ids_importers_and_a_resource_table() {
    let dir = scratch("flow");
    fs::write(dir.join("red.bmp"), tiny_bmp()).expect("write image");
    write_meta(&dir, "red.bmp", "00000000000000aa", "bmp");
    let source = FsSource::new(&dir);
    let catalog = AssetCatalog::scan(&source).expect("catalog");
    let loader = AssetLoader::new(Arc::new(source), Arc::new(BackgroundPool::new(1)));
    let importers = Arc::new(Importers::standard().expect("table").images);

    let red = catalog.by_path(&AssetPath::new("red.bmp")).expect("known");
    let image: ImageAsset = loader
        .load_imported(red.path.clone(), importers)
        .wait()
        .expect("imported");
    assert_eq!((image.width, image.height), (1, 1));

    // The loading code owns the pool; the table only records identity.
    let mut table = ResourceIdTable::<_, ImageAsset>::declaring(catalog.iter().map(|e| e.id));
    let mut images = ResourcePool::new();
    let id = ResourceId::new(images.insert(image));
    table.bind(red.id, id).expect("bind");
    assert_eq!(table.resource(red.id), Some(id));
    assert_eq!(table.key_of(id), Some(red.id));
    fs::remove_dir_all(&dir).expect("remove scratch dir");
}

#[test]
fn an_unregistered_importer_name_is_a_typed_error() {
    let dir = scratch("unknown");
    fs::write(dir.join("red.bmp"), tiny_bmp()).expect("write image");
    write_meta(&dir, "red.bmp", "00000000000000aa", "tga");
    let loader = AssetLoader::new(
        Arc::new(FsSource::new(&dir)),
        Arc::new(BackgroundPool::new(1)),
    );
    let importers = Arc::new(Importers::standard().expect("table").images);

    let result = loader
        .load_imported(AssetPath::new("red.bmp"), importers)
        .wait();

    assert!(matches!(
        result,
        Err(LoadError::Decode(ImportError::UnknownImporter(_)))
    ));
    fs::remove_dir_all(&dir).expect("remove scratch dir");
}
