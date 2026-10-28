//! Criterion benches for entity lifecycle, component migration and queries.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use meridian_ecs::{Component, Parent, Transform, World, propagate_transforms};

struct Position(f32);
impl Component for Position {}
struct Velocity;
impl Component for Velocity {}

fn populated_world(count: u16) -> World {
    let mut world = World::new();
    for i in 0..count {
        let entity = world.spawn();
        world.insert(entity, Position(f32::from(i)));
        world.insert(entity, Velocity);
    }
    world
}

fn spawn_with_components(c: &mut Criterion) {
    c.bench_function("spawn_insert_1000", |b| {
        b.iter(|| black_box(populated_world(1000)));
    });
}

fn query_positions(c: &mut Criterion) {
    let world = populated_world(10_000);
    c.bench_function("query_positions_10000", |b| {
        b.iter(|| world.query::<Position>().map(|(_, p)| p.0).sum::<f32>());
    });
}

fn query_many_positions_velocities(c: &mut Criterion) {
    let mut world = populated_world(10_000);
    c.bench_function("query_many_mut_position_velocity_10000", |b| {
        b.iter(|| {
            world
                .query_many_mut::<(&mut Position, &Velocity)>()
                .map(|(_, (p, _))| p.0)
                .sum::<f32>()
        });
    });
}

/// `count` entities in groups of ten under one root each.
fn hierarchy_world(count: u16) -> World {
    let mut world = World::new();
    let mut root = world.spawn();
    for i in 0..count {
        let entity = world.spawn();
        world.insert(entity, Transform::default());
        if i % 10 == 0 {
            root = entity;
        } else {
            world.insert(entity, Parent(root));
        }
    }
    world
}

fn propagate(c: &mut Criterion) {
    let mut world = hierarchy_world(10_000);
    propagate_transforms(&mut world);
    c.bench_function("propagate_transforms_10000_unchanged", |b| {
        b.iter(|| propagate_transforms(black_box(&mut world)));
    });
    c.bench_function("propagate_transforms_10000_first_pass", |b| {
        b.iter_batched(
            || hierarchy_world(10_000),
            |mut fresh| propagate_transforms(&mut fresh),
            criterion::BatchSize::LargeInput,
        );
    });
}

criterion_group!(
    benches,
    spawn_with_components,
    query_positions,
    query_many_positions_velocities,
    propagate
);
criterion_main!(benches);
