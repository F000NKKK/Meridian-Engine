//! Packs and sidecars against the loader.

use std::sync::Arc;

use meridian_asset_domain::{AssetId, PackPath, PackWriter, ShaderSourceDecoder};

use crate::*;

/// Runs a job right away on the calling thread.
struct Inline;

impl Spawn for Inline {
    fn spawn(&self, _priority: Priority, job: Box<dyn FnOnce() + Send>) {
        job();
    }
}

#[test]
fn a_sidecar_is_the_asset_path_plus_meta() {
    assert_eq!(
        AssetPath::new("meshes/crate.obj").sidecar(),
        AssetPath::new("meshes/crate.obj.meta")
    );
}

#[test]
fn a_pack_serves_assets_through_the_byte_source_port() {
    let mut writer = PackWriter::new();
    writer
        .add(
            AssetId::from_bits(1),
            PackPath::new("actors/hero.mprefab").unwrap(),
            b"prefab",
        )
        .unwrap();
    let mut bytes = writer.finish();
    let source = PackSource::open(bytes.clone()).unwrap();
    assert_eq!(
        source.read(&AssetPath::new("actors/hero.mprefab")),
        Ok(b"prefab".to_vec())
    );
    let missing = AssetPath::new("actors/nope.mprefab");
    assert_eq!(source.read(&missing), Err(SourceError::NotFound(missing)));
    let bad = AssetPath::new("/abs");
    assert_eq!(source.read(&bad), Err(SourceError::Rejected(bad)));

    // A damaged asset is unreadable, with the pack's own words.
    bytes[34] ^= 1;
    let damaged = PackSource::open(bytes).unwrap();
    let path = AssetPath::new("actors/hero.mprefab");
    assert!(
        matches!(damaged.read(&path), Err(SourceError::Unreadable { reason, .. }) if reason.contains("damaged"))
    );
}

#[test]
fn a_pack_source_loads_through_the_loader() {
    // The loader and the pack meet only at the ByteSource port.
    let mut writer = PackWriter::new();
    writer
        .add(
            AssetId::from_bits(9),
            PackPath::new("a.wgsl").unwrap(),
            b"fn main() {}",
        )
        .unwrap();
    let source = PackSource::open(writer.finish()).unwrap();
    let loader = AssetLoader::new(Arc::new(source), Arc::new(Inline));
    let shader = loader
        .load(AssetPath::new("a.wgsl"), ShaderSourceDecoder)
        .wait()
        .unwrap();
    assert_eq!(shader.source, "fn main() {}");
}
