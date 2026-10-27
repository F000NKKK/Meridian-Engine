//! The real pieces together: files on disk, the background pool, the loader.

use std::fs;
use std::sync::Arc;

use meridian_asset_app::{AssetLoader, AssetPath, LoadError, SourceError};
use meridian_asset_domain::Decoder;
use meridian_asset_fs::FsSource;
use meridian_runtime_threads::BackgroundPool;

struct ByteSum;

impl Decoder<u64> for ByteSum {
    type Error = &'static str;
    fn decode(&self, bytes: &[u8]) -> Result<u64, Self::Error> {
        Ok(bytes.iter().map(|&b| u64::from(b)).sum())
    }
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("meridian-load-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

#[test]
fn files_load_in_the_background_and_decode() {
    let dir = scratch("ok");
    for index in 0..40_u8 {
        fs::write(dir.join(format!("{index}.bin")), [index, 1]).expect("write asset");
    }
    let loader = AssetLoader::new(
        Arc::new(FsSource::new(&dir)),
        Arc::new(BackgroundPool::new(4)),
    );

    let pending: Vec<_> = (0..40_u8)
        .map(|index| loader.load(AssetPath::new(format!("{index}.bin")), ByteSum))
        .collect();

    for (index, load) in (0..40_u64).zip(pending) {
        assert_eq!(load.wait(), Ok(index + 1));
    }
    fs::remove_dir_all(&dir).expect("remove scratch dir");
}

#[test]
fn a_missing_file_reports_not_found_through_the_pool() {
    let dir = scratch("missing");
    let loader = AssetLoader::new(
        Arc::new(FsSource::new(&dir)),
        Arc::new(BackgroundPool::new(2)),
    );
    let path = AssetPath::new("absent.bin");

    let result = loader.load(path.clone(), ByteSum).wait();

    assert_eq!(result, Err(LoadError::Source(SourceError::NotFound(path))));
    fs::remove_dir_all(&dir).expect("remove scratch dir");
}
