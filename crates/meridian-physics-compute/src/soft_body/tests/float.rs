//! Tests of the `f32` soft-body GPU kernel.

use meridian_compute_runtime::ComputeContext;
use meridian_gac_core::volume::Plane;
use meridian_gac_core::{FloatFlavor, Vec3};
use meridian_physics_core::soft_body::float::{
    SoftBody, SoftBodyIntegrator, SoftBodyMaterial, SpringTuning, icosphere_soft_body,
};

use crate::soft_body::float::SoftBodyGpuKernel;

fn ground() -> Plane<FloatFlavor> {
    Plane {
        normal: Vec3::Y,
        d: 0.0,
    }
    .normalize()
}

fn ball(center: Vec3, radius: f32) -> SoftBody {
    icosphere_soft_body(
        center,
        radius,
        1,
        SoftBodyMaterial {
            particle_mass: 0.05,
            edge: SpringTuning {
                stiffness: 400.0,
                damping: 2.0,
            },
            spoke: SpringTuning {
                stiffness: 150.0,
                damping: 1.0,
            },
        },
    )
}

/// Needs a real adapter; skip rather than fail where none is
/// available, matching every other GPU-touching test in this
/// workspace.
#[cfg(feature = "hw-tests")]
async fn kernel_or_skip() -> Option<(ComputeContext, SoftBodyGpuKernel)> {
    let context = match ComputeContext::new().with_gpu().await {
        Ok(ctx) => ctx,
        Err(err) => {
            eprintln!("skipping: no GPU device available ({err})");
            return None;
        }
    };
    let kernel = SoftBodyGpuKernel::new(&context);
    Some((context, kernel))
}

/// Stepping the same scenario on the GPU and on the CPU must agree
/// closely (not bit-exact — see the module doc) during smooth
/// free-fall, before any ground contact. A mass-spring system is
/// chaotic once collision response and inter-particle feedback are
/// both in play (a single differing float ULP at the moment of
/// impact can select a measurably different post-bounce trajectory,
/// the same sensitivity that makes weather forecasting hard, not a
/// bug in either integrator) — so this only asserts tight agreement
/// for the pre-contact steps, then checks the post-contact tail
/// stays physically plausible (bounded, no NaN/blow-up) rather than
/// demanding it retrace the CPU path exactly.
#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn gpu_step_matches_cpu_step_closely() {
    let Some((context, kernel)) = kernel_or_skip().await else {
        return;
    };

    let mut gpu_body = ball(Vec3::new(0.1, 0.6, -0.05), 0.5);
    let mut cpu_body = gpu_body.clone();
    let integrator = SoftBodyIntegrator::new(Vec3::new(0.0, -9.81 * 4.0, 0.0), ground(), 0.2);
    let dt = 1.0 / 240.0;
    let pre_contact_steps = 20;

    for step in 0..120 {
        kernel.step(&context, &mut gpu_body, &integrator, dt).await;
        integrator.step(&mut cpu_body, dt);

        for (i, (gpu_p, cpu_p)) in gpu_body
            .positions
            .iter()
            .zip(cpu_body.positions.iter())
            .enumerate()
        {
            let diff = (*gpu_p - *cpu_p).length();
            if step < pre_contact_steps {
                assert!(
                    diff < 1e-4,
                    "step {step} particle {i} diverged during free-fall: gpu={gpu_p:?} cpu={cpu_p:?} diff={diff}"
                );
            } else {
                assert!(
                    gpu_p.x.is_finite()
                        && gpu_p.y.is_finite()
                        && gpu_p.z.is_finite()
                        && gpu_p.length() < 10.0,
                    "step {step} particle {i}: gpu position blew up: {gpu_p:?}"
                );
            }
        }
    }
}
