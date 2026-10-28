//! `FsSource` against a scratch directory.

use std::fs;
use std::path::PathBuf;

use meridian_asset_app::{AssetPath, ByteSource, SourceError};

use super::FsSource;

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
