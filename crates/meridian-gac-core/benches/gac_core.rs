//! Criterion benches for the geometric-algebra hot paths.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use meridian_gac_core::{Motor3, Rotor, Vec3};

fn rotor_transform(c: &mut Criterion) {
    let rotor = Rotor::from_axis_angle(Vec3::new(1.0, 2.0, 3.0), 0.7);
    let v = Vec3::new(0.3, -1.2, 4.5);
    c.bench_function("rotor_transform_vector", |b| {
        b.iter(|| black_box(rotor).transform_vector(black_box(v)));
    });
}

fn motor_compose(c: &mut Criterion) {
    let a = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), 0.3),
        Vec3::new(1.0, 2.0, 3.0),
    );
    let b2 = Motor3::translation(Vec3::new(-1.0, 0.5, 2.0));
    c.bench_function("motor_compose", |b| {
        b.iter(|| black_box(a).compose(black_box(b2)));
    });
    let p = Vec3::new(0.1, 0.2, 0.3);
    c.bench_function("motor_transform_point", |b| {
        b.iter(|| black_box(a).transform_point(black_box(p)));
    });
}

fn motor_batch(c: &mut Criterion) {
    let motor = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 0.3),
        Vec3::new(1.0, 2.0, 0.0),
    );
    let points: Vec<Vec3> = (0..10_000)
        .map(|i| Vec3::new(i as f32 * 0.01, (i % 97) as f32, 0.0))
        .collect();
    c.bench_function("motor_transform_point_x10000", |b| {
        b.iter(|| {
            let mut sum = Vec3::ZERO;
            for &p in &points {
                sum = sum + black_box(motor).transform_point(p);
            }
            sum
        });
    });
    c.bench_function("motor_transform_vector", |b| {
        b.iter(|| black_box(motor).transform_vector(black_box(Vec3::X)));
    });
    c.bench_function("motor_to_mat4", |b| b.iter(|| black_box(motor).to_mat4()));
}

criterion_group!(benches, rotor_transform, motor_compose, motor_batch);
criterion_main!(benches);
