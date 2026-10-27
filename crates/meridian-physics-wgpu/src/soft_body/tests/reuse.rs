//! One soft-body kernel reusing its GPU buffers across bodies of different sizes.

#![cfg(feature = "hw-tests")]

use meridian_numeric::Fixed;
use meridian_physics_domain::soft_body::fixed::{
    FixedSoftBody, FixedSoftBodyIntegrator, FixedSoftBodyMaterial, FixedSpringTuning,
    fixed_icosphere_soft_body,
};

use super::fixed::{ball, fv3, ground, kernel_or_skip};

/// One kernel stepping bodies of different sizes in turn reuses and grows
/// its buffers (a larger body reallocates, a smaller one reads back a
/// shorter prefix of the oversized buffers); every result must still match
/// the CPU bit for bit.
#[tokio::test]
async fn one_kernel_stays_exact_across_growing_and_shrinking_bodies() {
    let Some((context, kernel)) = kernel_or_skip().await else {
        return;
    };
    let integrator =
        FixedSoftBodyIntegrator::new(fv3(0.0, -9.81, 0.0), ground(), Fixed::from_num(0.3));
    let dt = Fixed::from_num(1.0 / 240.0);
    let bodies = [
        ball(fv3(0.0, 0.7, 0.0), 0.3),
        fixed_larger_ball(),
        ball(fv3(0.2, 0.9, 0.0), 0.3),
    ];

    for (round, mut gpu_body) in bodies.into_iter().enumerate() {
        let mut cpu_body = gpu_body.clone();
        for step in 0..5 {
            kernel.step(&context, &mut gpu_body, &integrator, dt).await;
            integrator.step(&mut cpu_body, dt);
            assert_eq!(
                gpu_body.positions, cpu_body.positions,
                "round {round} step {step}: positions diverged"
            );
            assert_eq!(
                gpu_body.velocities, cpu_body.velocities,
                "round {round} step {step}: velocities diverged"
            );
        }
    }
}

fn fixed_larger_ball() -> FixedSoftBody {
    fixed_icosphere_soft_body(
        fv3(0.0, 0.8, 0.0),
        Fixed::from_num(0.4),
        2,
        FixedSoftBodyMaterial {
            particle_mass: Fixed::from_num(0.05),
            edge: FixedSpringTuning {
                stiffness: Fixed::from_num(400.0),
                damping: Fixed::from_num(2.0),
            },
            spoke: FixedSpringTuning {
                stiffness: Fixed::from_num(150.0),
                damping: Fixed::from_num(1.0),
            },
        },
    )
}
