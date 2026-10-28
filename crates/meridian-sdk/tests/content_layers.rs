//! Choosing a directory, a pack or a stack of them at startup: the loader
//! and `WorldFiles` read all of them the same way.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use meridian_asset_domain::{AssetId, PackPath, PackWriter};
use meridian_sdk::content::{CatalogError, Importers, Layer, Layers, Mounted, OpenError};
use meridian_sdk::persistence::{
    Component, ComponentName, ComponentRegistry, EntityKey, NoResolver, World, WorldFiles,
};
use meridian_sdk::{AssetPath, LoadError, SourceError};
use substrate_data::Schema;

#[derive(Debug, Clone, PartialEq, Schema)]
struct Health {
    points: u32,
}
impl Component for Health {}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("meridian-sdk-cl-{name}-{}", std::process::id()));
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn prefab(points: u32) -> String {
    format!(
        r#"{{ format: 1, entities: [ {{ id: "body", components: {{ health: {{ points: {points} }} }} }} ] }}"#
    )
}

const SCENE: &str = r#"{ format: 1, entities: [ { id: "hero", prefab: "@prefab:hero" } ] }"#;

/// A pack file at `path` holding `files`, ids numbered from 1.
fn write_pack(path: &Path, files: &[(&str, &[u8])]) {
    let mut writer = PackWriter::new();
    for (index, (name, bytes)) in files.iter().enumerate() {
        let id = AssetId::from_bits(index as u64 + 1);
        writer
            .add(id, PackPath::new(name).expect("path"), bytes)
            .expect("unique");
    }
    fs::write(path, writer.finish()).expect("write pack");
}

fn world_files(content: &Mounted, saves: &Path) -> WorldFiles {
    let mut registry = ComponentRegistry::new();
    registry
        .register::<Health>(ComponentName::new("health").expect("name"))
        .expect("register");
    WorldFiles::new(saves, content.default_loader(), registry)
}

fn hero_health(files: &WorldFiles) -> u32 {
    let mut world = World::new();
    let scene = files
        .load_scene("level.mscene", &mut world, &mut NoResolver)
        .expect("loads");
    let key = EntityKey::new("hero").expect("key");
    let hero = scene.entity(&key).expect("hero");
    world.get::<Health>(hero).expect("health").points
}

#[test]
fn a_directory_and_a_pack_serve_the_same_scene() {
    let root = scratch("same");
    fs::create_dir_all(root.join("loose")).expect("mkdir");
    fs::write(root.join("loose/level.mscene"), SCENE).expect("write");
    fs::write(root.join("loose/hero.mprefab"), prefab(80)).expect("write");
    let pack = root.join("content.mpak");
    let hero = prefab(80);
    write_pack(
        &pack,
        &[
            ("level.mscene", SCENE.as_bytes()),
            ("hero.mprefab", hero.as_bytes()),
        ],
    );

    let from_dir = Mounted::mount(Layers::directory(root.join("loose"))).expect("directory");
    let from_pack = Mounted::mount(Layers::pack(&pack)).expect("pack");

    assert_eq!(hero_health(&world_files(&from_dir, &root)), 80);
    assert_eq!(hero_health(&world_files(&from_pack, &root)), 80);
    fs::remove_dir_all(&root).expect("remove");
}

#[test]
fn the_later_layer_overrides_the_earlier_one() {
    let root = scratch("overlay");
    fs::create_dir_all(root.join("loose")).expect("mkdir");
    fs::write(root.join("loose/hero.mprefab"), prefab(5)).expect("write");
    let pack = root.join("content.mpak");
    let hero = prefab(80);
    write_pack(
        &pack,
        &[
            ("level.mscene", SCENE.as_bytes()),
            ("hero.mprefab", hero.as_bytes()),
        ],
    );

    let loose_over_pack = Layers::pack(&pack).over(Layer::Directory(root.join("loose")));
    let pack_over_loose = Layers::new(Layer::Directory(root.join("loose"))).over(Layer::Pack(pack));

    let a = Mounted::mount(loose_over_pack).expect("mounts");
    let b = Mounted::mount(pack_over_loose).expect("mounts");
    assert_eq!(hero_health(&world_files(&a, &root)), 5);
    assert_eq!(hero_health(&world_files(&b, &root)), 80);
    fs::remove_dir_all(&root).expect("remove");
}

#[test]
fn a_catalog_and_load_imported_work_over_a_pack() {
    let root = scratch("catalog");
    let pack = root.join("content.mpak");
    let meta = br#"{ format: 1, id: "00000000000000aa", importer: "obj" }"#;
    write_pack(
        &pack,
        &[
            ("tri.obj", b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n"),
            ("tri.obj.meta", meta),
        ],
    );
    let content = Mounted::mount(Layers::pack(&pack)).expect("pack");

    let catalog = content.catalog().expect("catalog");
    let entry = catalog.by_path(&AssetPath::new("tri.obj")).expect("known");
    assert_eq!(entry.id, AssetId::from_bits(0xaa));
    let meshes = Arc::new(Importers::standard().expect("table").meshes);
    let mesh = content
        .default_loader()
        .load_imported(entry.path.clone(), meshes)
        .wait()
        .expect("imported");
    assert_eq!(mesh.indices.len(), 3);
    fs::remove_dir_all(&root).expect("remove");
}

#[test]
fn a_missing_asset_is_not_found_in_every_layout() {
    let root = scratch("missing");
    let pack = root.join("content.mpak");
    write_pack(&pack, &[("a.bin", b"a")]);
    for layers in [Layers::directory(&root), Layers::pack(&pack)] {
        let result = Mounted::mount(layers)
            .expect("mounts")
            .default_loader()
            .load(AssetPath::new("nope.bin"), Bytes)
            .wait();
        assert!(matches!(
            result,
            Err(LoadError::Source(SourceError::NotFound(_)))
        ));
    }
    fs::remove_dir_all(&root).expect("remove");
}

struct Bytes;
impl meridian_sdk::Decoder<Vec<u8>> for Bytes {
    type Error = std::convert::Infallible;
    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Ok(bytes.to_vec())
    }
}

#[test]
fn bad_layers_are_typed_errors() {
    let root = scratch("bad");
    let not_a_dir = Mounted::mount(Layers::directory(root.join("nope"))).expect_err("missing");
    assert!(matches!(not_a_dir, OpenError::NotADirectory(_)));
    let no_file = Mounted::mount(Layers::pack(root.join("nope.mpak"))).expect_err("missing");
    assert!(matches!(no_file, OpenError::File { .. }));
    fs::write(root.join("junk.mpak"), b"not a pack at all, really not").expect("write");
    let junk = Mounted::mount(Layers::pack(root.join("junk.mpak"))).expect_err("junk");
    assert!(matches!(junk, OpenError::Pack { .. }));
    // One bad layer anywhere refuses the whole stack.
    let stack = Layers::directory(&root).over(Layer::Pack(root.join("junk.mpak")));
    assert!(Mounted::mount(stack).is_err());
    fs::remove_dir_all(&root).expect("remove");
}

#[test]
fn a_truncated_or_flipped_pack_never_panics_and_is_never_read_as_intact() {
    let root = scratch("damage");
    let pack = root.join("content.mpak");
    write_pack(
        &pack,
        &[("a.bin", b"payload-a"), ("b/c.bin", b"payload-bc")],
    );
    let good = fs::read(&pack).expect("read");
    let damaged = root.join("damaged.mpak");

    for cut in 0..good.len() {
        fs::write(&damaged, &good[..cut]).expect("write");
        assert!(Mounted::mount(Layers::pack(&damaged)).is_err(), "cut {cut}");
    }
    for at in 0..good.len() {
        let mut bytes = good.clone();
        bytes[at] ^= 0xff;
        fs::write(&damaged, &bytes).expect("write");
        let outcome = Mounted::mount(Layers::pack(&damaged)).map(|content| {
            ["a.bin", "b/c.bin"]
                .iter()
                .all(|name| content.source().read(&AssetPath::new(*name)).is_ok())
        });
        assert!(
            matches!(outcome, Err(_) | Ok(false)),
            "byte {at} went unnoticed"
        );
    }
    fs::remove_dir_all(&root).expect("remove");
}

#[test]
fn a_catalog_over_layers_reports_a_duplicate_id() {
    let root = scratch("dup");
    let meta = |id: &str| format!(r#"{{ format: 1, id: "{id}", importer: "obj" }}"#);
    fs::write(root.join("a.obj.meta"), meta("0000000000000001")).expect("write");
    fs::write(root.join("b.obj.meta"), meta("0000000000000001")).expect("write");
    let content = Mounted::mount(Layers::directory(&root)).expect("dir");
    assert!(matches!(
        content.catalog(),
        Err(CatalogError::DuplicateId { .. })
    ));
    fs::remove_dir_all(&root).expect("remove");
}
