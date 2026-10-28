//! Hardware tests of the GPU broad-phase kernel.

use meridian_compute_wgpu::gpu_context;
use meridian_gac::{FloatFlavor, Vec3};
use meridian_physics_domain::dynamics::RigidBody;

use super::float::BroadPhaseGpuKernel;

fn sphere_at(x: f32) -> RigidBody<FloatFlavor> {
    RigidBody {
        frame: meridian_gac::Motor3::translation(Vec3::new(x, 0.0, 0.0)),
        velocity: Vec3::ZERO,
        angular_velocity: meridian_gac::Bivector3::ZERO,
        mass: 1.0,
        shape: meridian_physics_domain::collision::ColliderShape::Sphere { radius: 0.5 },
    }
}

/// The GPU kernel's overlapping pairs must match
/// `BroadPhase::find_candidate_pairs` exactly, in the same order —
/// same contract [`crate::broad_phase::BroadPhasePairsKernel`]'s own
/// CPU-batched test proves, just through a real GPU dispatch this
/// time instead of `parallel_for`.
#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn gpu_pairs_match_direct_find_candidate_pairs() {
    let context = match gpu_context().await {
        Ok(ctx) => ctx,
        Err(err) => {
            eprintln!("skipping: no GPU device available ({err})");
            return;
        }
    };
    let kernel = BroadPhaseGpuKernel::new(&context).expect("GPU context");

    let bodies = vec![
        sphere_at(0.0),
        sphere_at(0.6),  // overlaps 0
        sphere_at(10.0), // isolated
        sphere_at(10.4), // overlaps 2, not 0/1
    ];

    let mut broad_phase = meridian_physics_domain::collision::BroadPhase::<FloatFlavor>::new();
    let expected = broad_phase.find_candidate_pairs(&bodies).to_vec();

    let got = kernel.pairs(&context, &bodies).await.expect("GPU context");
    assert_eq!(got, expected);
}

/// Exercises a real, non-trivial overlap pattern above the 64-thread
/// workgroup size, so more than one workgroup dispatches.
#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn gpu_pairs_match_direct_find_candidate_pairs_large_batch() {
    let context = match gpu_context().await {
        Ok(ctx) => ctx,
        Err(err) => {
            eprintln!("skipping: no GPU device available ({err})");
            return;
        }
    };
    let kernel = BroadPhaseGpuKernel::new(&context).expect("GPU context");

    // 60 spheres, radius 0.5, spaced 0.4 apart -> every consecutive
    // pair overlaps, none else do -> 1770 candidate pairs.
    let bodies: Vec<_> = (0..60).map(|i| sphere_at(i as f32 * 0.4)).collect();

    let mut broad_phase = meridian_physics_domain::collision::BroadPhase::<FloatFlavor>::new();
    let expected = broad_phase.find_candidate_pairs(&bodies).to_vec();

    let got = kernel.pairs(&context, &bodies).await.expect("GPU context");
    assert_eq!(got, expected);
}
