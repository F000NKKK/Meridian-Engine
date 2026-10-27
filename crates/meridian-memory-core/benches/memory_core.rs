//! Criterion benches for the pool and arena allocation paths.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use meridian_memory_core::{FrameArena, ResourcePool};

fn pool_insert_remove(c: &mut Criterion) {
    c.bench_function("resource_pool_insert_remove_1000", |b| {
        b.iter(|| {
            let mut pool = ResourcePool::new();
            let handles: Vec<_> = (0..1000u32).map(|i| pool.insert(i)).collect();
            for handle in handles {
                black_box(pool.remove(handle));
            }
        });
    });
}

fn pool_lookup(c: &mut Criterion) {
    let mut pool = ResourcePool::new();
    let handles: Vec<_> = (0..1000u32).map(|i| pool.insert(i)).collect();
    c.bench_function("resource_pool_get_1000", |b| {
        b.iter(|| handles.iter().filter_map(|h| pool.get(*h)).sum::<u32>());
    });
}

fn frame_arena_cycle(c: &mut Criterion) {
    let mut arena = FrameArena::new();
    c.bench_function("frame_arena_alloc_reset_1000", |b| {
        b.iter(|| {
            for i in 0..1000u32 {
                black_box(arena.alloc(i));
            }
            arena.reset();
        });
    });
}

criterion_group!(benches, pool_insert_remove, pool_lookup, frame_arena_cycle);
criterion_main!(benches);
