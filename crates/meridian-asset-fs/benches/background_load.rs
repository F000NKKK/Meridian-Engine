//! Throughput of many small loads through the background pool.

use std::fs;
use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use meridian_asset_app::{AssetLoader, AssetPath};
use meridian_asset_domain::Decoder;
use meridian_asset_fs::FsSource;
use meridian_runtime_threads::BackgroundPool;

struct Length;

impl Decoder<usize> for Length {
    type Error = std::convert::Infallible;
    fn decode(&self, bytes: &[u8]) -> Result<usize, Self::Error> {
        Ok(bytes.len())
    }
}

fn small_loads(c: &mut Criterion) {
    let dir = std::env::temp_dir().join(format!("meridian-load-bench-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create bench dir");
    fs::write(dir.join("a.bin"), [0_u8; 256]).expect("write asset");
    let loader = AssetLoader::new(
        Arc::new(FsSource::new(&dir)),
        Arc::new(BackgroundPool::new(4)),
    );

    c.bench_function("load_1000_small_files", |b| {
        b.iter(|| {
            let pending: Vec<_> = (0..1000)
                .map(|_| loader.load(AssetPath::new("a.bin"), Length))
                .collect();
            for load in pending {
                black_box(load.wait()).expect("load");
            }
        });
    });
    fs::remove_dir_all(&dir).expect("remove bench dir");
}

criterion_group!(benches, small_loads);
criterion_main!(benches);
