//! Criterion benches for the CPU half of the sprite path.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use meridian_gac_core::{Motor3, Vec3};
use meridian_graphics_core::scene::{Camera2d, Scene2d, Sprite};
use meridian_graphics_core::{TextureHandle, batch_scene2d};
use meridian_memory_core::ResourcePool;

fn ui_like_scene(count: u16) -> Scene2d {
    let mut pool = ResourcePool::<()>::default();
    let textures: Vec<TextureHandle> = (0..4)
        .map(|_| TextureHandle::new(pool.insert(())))
        .collect();
    let sprites = (0..count)
        .map(|i| Sprite {
            texture: textures[usize::from(i / 64 % 4)],
            source_rect: None,
            size: [24.0, 24.0],
            tint: [1.0; 4],
            frame: Motor3::translation(Vec3::new(
                f32::from(i % 80) * 12.0,
                f32::from(i / 80) * 12.0,
                0.0,
            )),
            layer: i32::from(i),
        })
        .collect();
    Scene2d {
        camera: Camera2d::from_surface_size(960.0, 720.0),
        sprites,
    }
}

fn batching(c: &mut Criterion) {
    let scene = ui_like_scene(10_000);
    c.bench_function("batch_scene2d_10000_sprites", |b| {
        b.iter(|| black_box(batch_scene2d(black_box(&scene))));
    });
}

fn transform_only(c: &mut Criterion) {
    let scene = ui_like_scene(10_000);
    c.bench_function("motor3_transform_point_10000", |b| {
        b.iter(|| {
            let mut acc = 0.0f32;
            for s in &scene.sprites {
                acc += s.frame.transform_point(Vec3::ZERO).x;
            }
            black_box(acc)
        });
    });
}

criterion_group!(benches, batching, transform_only);
criterion_main!(benches);
