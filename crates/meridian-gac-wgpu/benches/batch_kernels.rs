//! Criterion benches for the batch motor kernels: the same dispatch run
//! sequentially and in parallel, so the parallel threshold is chosen on
//! numbers (`docs/segments/core.md`, backlog item 8).

use std::hint::black_box;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use meridian_compute_app::{ComputeScheduler, DispatchSize};
use meridian_compute_wgpu::CpuComputeDevice;
use meridian_gac::{FixedMotor3, FixedRotor, FixedVec3, Motor3, Rotor, Vec3};
use meridian_gac_wgpu::{FixedMotorTransformKernel, MotorTransformKernel};
use meridian_numeric::Fixed;

const SIZES: [u32; 4] = [1_000, 4_000, 10_000, 100_000];

fn schedulers() -> [(&'static str, ComputeScheduler); 2] {
    let sequential =
        ComputeScheduler::with_parallel_threshold(Arc::new(CpuComputeDevice::new()), usize::MAX);
    let parallel = ComputeScheduler::with_parallel_threshold(Arc::new(CpuComputeDevice::new()), 1);
    [("sequential", sequential), ("parallel", parallel)]
}

/// A small deterministic variation per index, exact in `f32` and `f64`.
fn wobble(index: u32) -> u16 {
    u16::try_from(index % 1024).unwrap_or_default()
}

fn float_locals(count: u32) -> Vec<Motor3> {
    (0..count)
        .map(|i| {
            Motor3::from_rotation_translation(
                Rotor::from_axis_angle(Vec3::Y, f32::from(wobble(i)) * 0.001),
                Vec3::new(0.0, f32::from(wobble(i)) * 0.01, 0.0),
            )
        })
        .collect()
}

fn fixed_locals(count: u32) -> Vec<FixedMotor3> {
    let axis = FixedVec3::new(Fixed::ZERO, Fixed::ONE, Fixed::ZERO);
    (0..count)
        .map(|i| {
            FixedMotor3::from_rotation_translation(
                FixedRotor::from_axis_angle(axis, Fixed::from_num(f64::from(wobble(i)) * 0.0001)),
                FixedVec3::new(
                    Fixed::ZERO,
                    Fixed::from_num(f64::from(wobble(i)) * 0.001),
                    Fixed::ZERO,
                ),
            )
        })
        .collect()
}

fn motor_transform(c: &mut Criterion) {
    let parent = Motor3::translation(Vec3::new(10.0, 0.0, 0.0));
    let mut group = c.benchmark_group("motor_transform_kernel");
    for size in SIZES {
        for (name, scheduler) in schedulers() {
            let kernel = MotorTransformKernel::new(parent, float_locals(size));
            group.bench_with_input(BenchmarkId::new(name, size), &size, |b, &size| {
                b.iter(|| scheduler.run(black_box(&kernel), DispatchSize::linear(size)));
            });
        }
    }
    group.finish();
}

fn fixed_motor_transform(c: &mut Criterion) {
    let parent = FixedMotor3::translation(FixedVec3::new(
        Fixed::from_num(10.0),
        Fixed::ZERO,
        Fixed::ZERO,
    ));
    let mut group = c.benchmark_group("fixed_motor_transform_kernel");
    for size in SIZES {
        for (name, scheduler) in schedulers() {
            let kernel = FixedMotorTransformKernel::new(parent, fixed_locals(size));
            group.bench_with_input(BenchmarkId::new(name, size), &size, |b, &size| {
                b.iter(|| scheduler.run(black_box(&kernel), DispatchSize::linear(size)));
            });
        }
    }
    group.finish();
}

criterion_group!(benches, motor_transform, fixed_motor_transform);
criterion_main!(benches);
