//! Criterion benches for the geometric-algebra hot paths.

use criterion::{Criterion, criterion_group, criterion_main};
use meridian_gac_core::{Motor3, Rotor, Vec3};
use std::hint::black_box;

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

criterion_group!(benches, rotor_transform, motor_compose);
criterion_main!(benches);
