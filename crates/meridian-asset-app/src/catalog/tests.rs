//! Catalog scans over an in-memory source.

use meridian_asset_domain::{AssetId, AssetMeta, ImporterName};

use super::{AssetCatalog, CatalogError};
use crate::source::MemorySource;
use crate::{AssetPath, MetaReadError};

fn id(bits: u64) -> AssetId {
    AssetId::from_bits(bits)
}

fn meta(own: u64, depends_on: &[u64]) -> Vec<u8> {
    let mut meta = AssetMeta::new(id(own), ImporterName::new("obj").unwrap());
    meta.dependencies = depends_on.iter().map(|&other| id(other)).collect();
    meta.to_bytes()
}

fn path(text: &str) -> AssetPath {
    AssetPath::new(text)
}

#[test]
fn scans_sidecars_into_ids_and_paths() {
    let source = MemorySource::default()
        .with("meshes/crate.obj", b"v")
        .with("meshes/crate.obj.meta", &meta(1, &[2]))
        .with("tex/grass.png.meta", &meta(2, &[]))
        .with("scene.mscene", b"{}");
    let catalog = AssetCatalog::scan(&source).unwrap();

    assert_eq!(catalog.len(), 2);
    assert_eq!(catalog.id_of(&path("meshes/crate.obj")), Some(id(1)));
    let crate_obj = catalog.get(id(1)).unwrap();
    assert_eq!(crate_obj.path, path("meshes/crate.obj"));
    assert_eq!(crate_obj.importer.as_str(), "obj");
    assert_eq!(crate_obj.dependencies, vec![id(2)]);
    assert_eq!(catalog.by_path(&path("tex/grass.png")).unwrap().id, id(2));
    assert!(catalog.id_of(&path("scene.mscene")).is_none());
    let order: Vec<_> = catalog.iter().map(|entry| entry.id).collect();
    assert_eq!(order, vec![id(1), id(2)]);
}

#[test]
fn a_source_without_sidecars_is_an_empty_catalog() {
    let catalog = AssetCatalog::scan(&MemorySource::default().with("a.bin", b"x")).unwrap();
    assert!(catalog.is_empty());
}

#[test]
fn two_assets_with_one_id_are_refused() {
    let source = MemorySource::default()
        .with("a.obj.meta", &meta(1, &[]))
        .with("b.obj.meta", &meta(1, &[]));
    let error = AssetCatalog::scan(&source).unwrap_err();
    assert_eq!(
        error,
        CatalogError::DuplicateId {
            id: id(1),
            first: path("a.obj"),
            second: path("b.obj"),
        }
    );
}

#[test]
fn a_dependency_nobody_declares_is_refused() {
    let source = MemorySource::default().with("a.obj.meta", &meta(1, &[9]));
    assert_eq!(
        AssetCatalog::scan(&source).unwrap_err(),
        CatalogError::MissingDependency {
            asset: path("a.obj"),
            dependency: id(9),
        }
    );
}

#[test]
fn a_dependency_loop_is_refused() {
    let source = MemorySource::default()
        .with("a.obj.meta", &meta(1, &[2]))
        .with("b.obj.meta", &meta(2, &[3]))
        .with("c.obj.meta", &meta(3, &[1]));
    assert!(matches!(
        AssetCatalog::scan(&source).unwrap_err(),
        CatalogError::DependencyCycle { .. }
    ));
}

#[test]
fn a_diamond_is_not_a_loop() {
    let source = MemorySource::default()
        .with("a.obj.meta", &meta(1, &[2, 3]))
        .with("b.obj.meta", &meta(2, &[4]))
        .with("c.obj.meta", &meta(3, &[4]))
        .with("d.obj.meta", &meta(4, &[]));
    assert_eq!(AssetCatalog::scan(&source).unwrap().len(), 4);
}

#[test]
fn a_damaged_sidecar_names_its_file() {
    let good = meta(1, &[]);
    let mut source = MemorySource::default().with("ok.obj.meta", &good);
    for cut in 0..good.len() {
        source = source.with("bad.obj.meta", &good[..cut]);
        let error = AssetCatalog::scan(&source).unwrap_err();
        assert!(
            matches!(&error, CatalogError::Meta(MetaReadError::Invalid { path, .. })
                if path.as_str() == "bad.obj.meta"),
            "cut at {cut}: {error:?}"
        );
    }
}

#[test]
fn a_flipped_sidecar_byte_never_panics() {
    let good = meta(1, &[]);
    for at in 0..good.len() {
        let mut bad = good.clone();
        bad[at] ^= 0xff;
        let source = MemorySource::default().with("a.obj.meta", &bad);
        drop(AssetCatalog::scan(&source));
    }
}
