//! `FsSource` against a scratch directory.

use std::fs;
use std::path::PathBuf;

use meridian_asset_app::{AssetListing, AssetPath, ByteSource, SourceError};

use super::{FileReadAt, FsSource};

/// A fresh empty directory under the system temp dir, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    /// `test` keeps the directories of parallel tests apart.
    fn new(test: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("meridian-asset-fs-{}-{test}", std::process::id()));
        fs::create_dir_all(&dir).expect("create scratch dir");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let removed = fs::remove_dir_all(&self.0);
        debug_assert!(removed.is_ok(), "scratch dir removal");
    }
}

#[test]
fn reads_a_file_under_the_root_including_subdirectories() {
    let scratch = Scratch::new("reads");
    fs::create_dir_all(scratch.0.join("textures")).expect("mkdir");
    fs::write(scratch.0.join("textures/grass.bin"), [7, 8, 9]).expect("write");

    let source = FsSource::new(&scratch.0);
    let bytes = source.read(&AssetPath::new("textures/grass.bin"));
    assert_eq!(bytes, Ok(vec![7, 8, 9]));
}

#[test]
fn a_missing_file_is_not_found() {
    let scratch = Scratch::new("missing");
    let path = AssetPath::new("nope.bin");
    assert_eq!(
        FsSource::new(&scratch.0).read(&path),
        Err(SourceError::NotFound(path))
    );
}

#[test]
fn paths_that_leave_the_root_are_rejected_without_touching_the_disk() {
    let scratch = Scratch::new("rejected");
    let source = FsSource::new(&scratch.0);
    for text in [
        "../outside",
        "a/../../outside",
        "/etc/passwd",
        "a/./b/../../..",
    ] {
        let path = AssetPath::new(text);
        assert_eq!(
            source.read(&path),
            Err(SourceError::Rejected(path)),
            "{text}"
        );
    }
}

#[test]
fn a_directory_is_unreadable_not_not_found() {
    let scratch = Scratch::new("directory");
    fs::create_dir_all(scratch.0.join("dir")).expect("mkdir");
    let result = FsSource::new(&scratch.0).read(&AssetPath::new("dir"));
    assert!(
        matches!(result, Err(SourceError::Unreadable { .. })),
        "{result:?}"
    );
}

#[test]
fn a_pack_file_is_read_by_range_through_the_file_adapter() {
    use meridian_asset_app::PackSource;
    use meridian_asset_domain::{AssetId, PackPath, PackWriter};

    let scratch = Scratch::new("pack");
    let mut writer = PackWriter::new();
    let crate_path = PackPath::new("meshes/crate.obj").unwrap();
    writer
        .add(AssetId::from_bits(1), crate_path, b"mesh bytes")
        .unwrap();
    writer
        .add(
            AssetId::from_bits(2),
            PackPath::new("a.bin").unwrap(),
            &[5_u8; 1000],
        )
        .unwrap();
    fs::write(scratch.0.join("content.mpak"), writer.finish()).expect("write pack");

    let file = FileReadAt::open(scratch.0.join("content.mpak")).expect("open");
    let source = PackSource::open(file).expect("a pack");
    assert_eq!(
        source.read(&AssetPath::new("meshes/crate.obj")),
        Ok(b"mesh bytes".to_vec())
    );
    assert_eq!(source.read(&AssetPath::new("a.bin")), Ok(vec![5_u8; 1000]));
    let missing = AssetPath::new("nope.bin");
    assert_eq!(source.read(&missing), Err(SourceError::NotFound(missing)));
    let escaping = AssetPath::new("../x");
    assert_eq!(source.read(&escaping), Err(SourceError::Rejected(escaping)));
}

#[test]
fn a_missing_pack_file_is_a_typed_error() {
    let scratch = Scratch::new("nopack");
    assert!(FileReadAt::open(scratch.0.join("nope.mpak")).is_err());
}

#[test]
fn reads_past_the_end_are_out_of_range() {
    use meridian_asset_domain::{ReadAt, ReadAtError};
    let scratch = Scratch::new("range");
    fs::write(scratch.0.join("f"), [1, 2, 3]).expect("write");
    let file = FileReadAt::open(scratch.0.join("f")).expect("open");
    let mut buf = [0_u8; 2];
    file.read_exact_at(1, &mut buf).unwrap();
    assert_eq!(buf, [2, 3]);
    assert!(matches!(
        file.read_exact_at(2, &mut buf),
        Err(ReadAtError::OutOfRange { .. })
    ));
}

#[test]
fn lists_every_file_below_the_root_sorted_with_slashes() {
    let scratch = Scratch::new("listing");
    fs::create_dir_all(scratch.0.join("meshes/props")).expect("mkdir");
    fs::write(scratch.0.join("b.bin"), [1]).expect("write");
    fs::write(scratch.0.join("meshes/crate.obj"), [2]).expect("write");
    fs::write(scratch.0.join("meshes/props/lamp.obj"), [3]).expect("write");
    fs::create_dir_all(scratch.0.join("empty")).expect("mkdir");

    let paths = FsSource::new(&scratch.0).paths().expect("walked");

    let texts: Vec<_> = paths.iter().map(AssetPath::as_str).collect();
    assert_eq!(
        texts,
        ["b.bin", "meshes/crate.obj", "meshes/props/lamp.obj"]
    );
}

#[test]
fn listing_a_missing_root_is_unreadable_not_a_panic() {
    let scratch = Scratch::new("listing-missing");
    let source = FsSource::new(scratch.0.join("nope"));
    assert!(matches!(
        source.paths(),
        Err(SourceError::Unreadable { .. })
    ));
}

#[cfg(unix)]
#[test]
fn listing_does_not_follow_symbolic_links() {
    let scratch = Scratch::new("listing-links");
    fs::write(scratch.0.join("a.bin"), [1]).expect("write");
    std::os::unix::fs::symlink(&scratch.0, scratch.0.join("loop")).expect("symlink");

    let paths = FsSource::new(&scratch.0).paths().expect("walked");

    assert_eq!(paths, [AssetPath::new("a.bin")]);
}
