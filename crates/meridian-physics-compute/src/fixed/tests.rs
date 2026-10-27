//! Tests of this module.

use super::*;
use meridian_physics_core::soft_body::fixed::{
    FixedSoftBodyIntegrator, FixedSoftBodyMaterial, FixedSpringTuning, fixed_icosphere_soft_body,
};

fn fv3(x: f64, y: f64, z: f64) -> FixedVec3 {
    FixedVec3::new(Fixed::from_num(x), Fixed::from_num(y), Fixed::from_num(z))
}

fn ground() -> Plane<FixedFlavor> {
    Plane {
        normal: fv3(0.0, 1.0, 0.0),
        d: Fixed::ZERO,
    }
    .normalize()
}

fn ball(center: FixedVec3, radius: f64) -> FixedSoftBody {
    fixed_icosphere_soft_body(
        center,
        Fixed::from_num(radius),
        1,
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

/// Needs a real adapter; skip rather than fail where none is
/// available, matching every other GPU-touching test in this
/// workspace.
#[cfg(feature = "hw-tests")]
async fn kernel_or_skip() -> Option<(ComputeContext, FixedSoftBodyGpuKernel)> {
    let context = match ComputeContext::new().with_gpu().await {
        Ok(ctx) => ctx,
        Err(err) => {
            eprintln!("skipping: no GPU device available ({err})");
            return None;
        }
    };
    let kernel = FixedSoftBodyGpuKernel::new(&context);
    Some((context, kernel))
}

/// The actual point of this module: unlike `float`'s GPU/CPU
/// agreement (numerically close), the `Fixed` GPU path must be
/// bit-for-bit identical to `FixedSoftBodyIntegrator::step` — every
/// position and velocity component, every step, no drift. A gentle
/// drop at a fine timestep, matching
/// `soft_body::fixed`'s own reproducibility test, to stay inside
/// explicit-Euler's stable regime (see that test's own comment).
#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn gpu_step_matches_cpu_step_bit_exact() {
    let Some((context, kernel)) = kernel_or_skip().await else {
        return;
    };

    let mut gpu_body = ball(fv3(0.3, 0.7, -0.2), 0.5);
    let mut cpu_body = gpu_body.clone();
    let integrator =
        FixedSoftBodyIntegrator::new(fv3(0.0, -9.81, 0.0), ground(), Fixed::from_num(0.3));
    let dt = Fixed::from_num(1.0 / 240.0);

    for step in 0..60 {
        kernel
            .step(
                &context,
                &mut gpu_body,
                integrator.gravity,
                integrator.ground,
                integrator.restitution,
                dt,
            )
            .await;
        integrator.step(&mut cpu_body, dt);

        for (idx, (g, c)) in gpu_body
            .positions
            .iter()
            .zip(cpu_body.positions.iter())
            .enumerate()
        {
            assert_eq!(g, c, "step {step} particle {idx}: position diverged");
        }
        for (idx, (g, c)) in gpu_body
            .velocities
            .iter()
            .zip(cpu_body.velocities.iter())
            .enumerate()
        {
            if g != c {
                let degree = (
                    gpu_body_adjacency_degree(&gpu_body, idx),
                    idx,
                    gpu_body.particle_count(),
                );
                panic!(
                    "step {step} particle {idx}: velocity diverged gpu={g:?} cpu={c:?} degree/idx/count={degree:?}"
                );
            }
        }
    }
}

fn gpu_body_adjacency_degree(body: &FixedSoftBody, particle: usize) -> usize {
    body.springs
        .iter()
        .filter(|s| s.a == particle || s.b == particle)
        .count()
}

/// Pure-Rust replica of the GPU kernel's per-particle gather
/// algorithm (no WGSL, no GPU) — proves the *reformulation itself*
/// (independent of its WGSL/GPU execution) is bit-exact with
/// `FixedSoftBodyIntegrator::step`. See `crate::generic`'s module doc
/// for why this needs the `is_a`-conditional canonical direction:
/// recomputing direction/`total` per endpoint diverges by one raw bit
/// at step 48, and this test is the regression test for that.
fn rust_gather_step(
    body: &FixedSoftBody,
    gravity: FixedVec3,
    ground: Plane<FixedFlavor>,
    restitution: Fixed,
    dt: Fixed,
) -> FixedSoftBody {
    let adjacency = build_adjacency(body);
    let mut out = body.clone();
    for i in 0..body.particle_count() {
        let inverse_mass = body.inverse_masses[i];
        if inverse_mass <= Fixed::ZERO {
            continue;
        }
        let mut force = FixedVec3::ZERO;
        let start = adjacency.offsets[i] as usize;
        let end = adjacency.offsets[i + 1] as usize;
        for e in start..end {
            let (j, is_a) = crate::generic::decode_neighbor(adjacency.neighbor[e]);
            let delta = if is_a {
                body.positions[j] - body.positions[i]
            } else {
                body.positions[i] - body.positions[j]
            };
            let dist = delta.length();
            let direction = if dist > Fixed::from_bits(4) {
                delta * (Fixed::ONE / dist)
            } else {
                FixedVec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO)
            };
            let rest_length = adjacency.rest_length[e];
            let stiffness = adjacency.stiffness[e];
            let damping = adjacency.damping[e];
            let stretch = dist - rest_length;
            let spring_force = direction * (stiffness * stretch);
            let relative_velocity = if is_a {
                body.velocities[j] - body.velocities[i]
            } else {
                body.velocities[i] - body.velocities[j]
            };
            let closing_speed = relative_velocity.dot(direction);
            let damping_force = direction * (damping * closing_speed);
            let total = spring_force + damping_force;
            force = force + if is_a { total } else { -total };
        }
        let acceleration = force * inverse_mass + gravity;
        let mut velocity = body.velocities[i] + acceleration * dt;
        let mut position = body.positions[i] + velocity * dt;
        let separation = ground.distance(position);
        if separation < Fixed::ZERO {
            position = position - ground.normal * separation;
            let normal_speed = velocity.dot(ground.normal);
            if normal_speed < Fixed::ZERO {
                velocity = velocity - ground.normal * (normal_speed * (Fixed::ONE + restitution));
            }
        }
        out.positions[i] = position;
        out.velocities[i] = velocity;
    }
    out
}

#[test]
fn rust_gather_matches_cpu_scatter_bit_exact() {
    let mut gather_body = ball(fv3(0.3, 0.7, -0.2), 0.5);
    let mut cpu_body = gather_body.clone();
    let integrator =
        FixedSoftBodyIntegrator::new(fv3(0.0, -9.81, 0.0), ground(), Fixed::from_num(0.3));
    let dt = Fixed::from_num(1.0 / 240.0);

    for step in 0..60 {
        gather_body = rust_gather_step(
            &gather_body,
            integrator.gravity,
            integrator.ground,
            integrator.restitution,
            dt,
        );
        integrator.step(&mut cpu_body, dt);

        for (idx, (g, c)) in gather_body
            .velocities
            .iter()
            .zip(cpu_body.velocities.iter())
            .enumerate()
        {
            assert_eq!(
                g, c,
                "step {step} particle {idx}: gather vs scatter velocity diverged"
            );
        }
    }
}
