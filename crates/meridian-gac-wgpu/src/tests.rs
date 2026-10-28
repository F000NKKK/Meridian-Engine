//! Tests of this module.

use std::f32::consts::PI;
use std::sync::Arc;

use meridian_compute_app::{ComputeScheduler, DispatchSize};
use meridian_compute_wgpu::CpuComputeDevice;
use meridian_gac::{FixedMotor3, FixedRotor, FixedVec3, Motor3, Rotor, Vec3};
use meridian_numeric::Fixed;

use super::*;

fn approx_eq(a: Motor3, b: Motor3, p: Vec3) -> bool {
    (a.transform_point(p) - b.transform_point(p)).length() < 1e-4
}

#[test]
fn motor_transform_kernel_matches_gac_core_compose_directly() {
    let parent = Motor3::from_rotation_translation(
        Rotor::from_axis_angle(Vec3::Z, PI / 4.0),
        Vec3::new(10.0, 0.0, 0.0),
    );
    let locals: Vec<Motor3> = (0..2_000)
        .map(|i| {
            Motor3::from_rotation_translation(
                Rotor::from_axis_angle(Vec3::Y, i as f32 * 0.001),
                Vec3::new(0.0, i as f32 * 0.01, 0.0),
            )
        })
        .collect();

    let kernel = MotorTransformKernel::new(parent, locals.clone());
    // Force the parallel path (the default threshold is 4096 and
    // 2000 items does not reach it) so this test keeps covering the parallel
    // path whatever the default is.
    let scheduler = ComputeScheduler::with_parallel_threshold(Arc::new(CpuComputeDevice::new()), 1);
    scheduler.run(&kernel, DispatchSize::linear(locals.len() as u32));

    let results = kernel.results();
    assert_eq!(results.len(), locals.len());

    let p = Vec3::new(1.0, 0.0, 0.0);
    for (i, (&local, &world)) in locals.iter().zip(results.iter()).enumerate() {
        let expected = local.compose(parent);
        assert!(approx_eq(world, expected, p), "mismatch at index {i}");
        // Cross-check against the same "rotate/translate child, then
        // parent" property the roadmap's parent/child hierarchy
        // milestone validated directly in gac.
        let via_steps = parent.transform_point(local.transform_point(p));
        assert!(
            (world.transform_point(p) - via_steps).length() < 1e-4,
            "index {i} disagrees with step-by-step composition"
        );
    }
}

#[test]
fn motor_compose_kernel_handles_independent_parents_per_pair() {
    let pairs: Vec<(Motor3, Motor3)> = (0..500)
        .map(|i| {
            let child = Motor3::translation(Vec3::new(i as f32, 0.0, 0.0));
            let parent = Motor3::from_rotation_translation(
                Rotor::from_axis_angle(Vec3::Z, i as f32 * 0.01),
                Vec3::new(0.0, i as f32, 0.0),
            );
            (child, parent)
        })
        .collect();

    let kernel = MotorComposeKernel::new(pairs.clone());
    let scheduler = ComputeScheduler::with_parallel_threshold(Arc::new(CpuComputeDevice::new()), 1);
    scheduler.run(&kernel, DispatchSize::linear(pairs.len() as u32));

    let results = kernel.results();
    let p = Vec3::ZERO;
    for (i, (&(child, parent), &world)) in pairs.iter().zip(results.iter()).enumerate() {
        let expected = child.compose(parent);
        assert!(approx_eq(world, expected, p), "mismatch at index {i}");
    }
}

#[test]
fn dispatch_size_smaller_than_batch_only_processes_that_many() {
    let locals = vec![Motor3::translation(Vec3::new(1.0, 0.0, 0.0)); 10];
    let kernel = MotorTransformKernel::new(Motor3::identity(), locals);
    let scheduler = ComputeScheduler::new(Arc::new(CpuComputeDevice::new()));
    scheduler.run(&kernel, DispatchSize::linear(3));

    let results = kernel.results();
    let p = Vec3::ZERO;
    for r in &results[0..3] {
        assert!((r.transform_point(p) - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-4);
    }
    for r in &results[3..] {
        assert_eq!(
            *r,
            Motor3::identity(),
            "untouched slots must stay at their initial identity value"
        );
    }
}

fn fixed_approx_eq(a: FixedMotor3, b: FixedMotor3, p: FixedVec3) -> bool {
    (a.transform_point(p) - b.transform_point(p)).length() < Fixed::from_num(1e-3)
}

#[test]
fn fixed_motor_transform_kernel_matches_gac_core_compose_directly() {
    let parent = FixedMotor3::from_rotation_translation(
        FixedRotor::from_axis_angle(
            FixedVec3::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE),
            Fixed::from_num(std::f64::consts::FRAC_PI_4),
        ),
        FixedVec3::new(Fixed::from_num(10.0), Fixed::ZERO, Fixed::ZERO),
    );
    let locals: Vec<FixedMotor3> = (0..500)
        .map(|i| {
            FixedMotor3::from_rotation_translation(
                FixedRotor::from_axis_angle(
                    FixedVec3::new(Fixed::ZERO, Fixed::ONE, Fixed::ZERO),
                    Fixed::from_num(i as f64 * 0.001),
                ),
                FixedVec3::new(Fixed::ZERO, Fixed::from_num(i as f64 * 0.01), Fixed::ZERO),
            )
        })
        .collect();

    let kernel = FixedMotorTransformKernel::new(parent, locals.clone());
    let scheduler = ComputeScheduler::with_parallel_threshold(Arc::new(CpuComputeDevice::new()), 1);
    scheduler.run(&kernel, DispatchSize::linear(locals.len() as u32));

    let results = kernel.results();
    assert_eq!(results.len(), locals.len());

    let p = FixedVec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO);
    for (i, (&local, &world)) in locals.iter().zip(results.iter()).enumerate() {
        let expected = local.compose(parent);
        assert!(fixed_approx_eq(world, expected, p), "mismatch at index {i}");
    }
}

#[test]
fn fixed_motor_compose_kernel_handles_independent_parents_per_pair() {
    let pairs: Vec<(FixedMotor3, FixedMotor3)> = (0..200)
        .map(|i| {
            let child = FixedMotor3::translation(FixedVec3::new(
                Fixed::from_num(i as f64),
                Fixed::ZERO,
                Fixed::ZERO,
            ));
            let parent = FixedMotor3::from_rotation_translation(
                FixedRotor::from_axis_angle(
                    FixedVec3::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE),
                    Fixed::from_num(i as f64 * 0.01),
                ),
                FixedVec3::new(Fixed::ZERO, Fixed::from_num(i as f64), Fixed::ZERO),
            );
            (child, parent)
        })
        .collect();

    let kernel = FixedMotorComposeKernel::new(pairs.clone());
    let scheduler = ComputeScheduler::with_parallel_threshold(Arc::new(CpuComputeDevice::new()), 1);
    scheduler.run(&kernel, DispatchSize::linear(pairs.len() as u32));

    let results = kernel.results();
    let p = FixedVec3::ZERO;
    for (i, (&(child, parent), &world)) in pairs.iter().zip(results.iter()).enumerate() {
        let expected = child.compose(parent);
        assert!(fixed_approx_eq(world, expected, p), "mismatch at index {i}");
    }
}
