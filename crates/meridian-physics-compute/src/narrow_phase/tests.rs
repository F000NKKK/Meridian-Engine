//! Tests of this module.

use meridian_gac_core::float_ga::FloatFlavor;
use meridian_gac_core::{Motor3, Vec3};
use meridian_physics_core::generic::ColliderShape;

use super::*;

fn sphere_at(x: f32) -> RigidBody<FloatFlavor> {
    RigidBody {
        frame: Motor3::translation(Vec3::new(x, 0.0, 0.0)),
        velocity: Vec3::ZERO,
        angular_velocity: meridian_gac_core::Bivector3::ZERO,
        mass: 1.0,
        shape: ColliderShape::Sphere { radius: 0.5 },
    }
}

/// The kernel's batched `test_pair` results, run through
/// `compute-runtime`'s CPU `parallel_for`, must match calling
/// `NarrowPhase::test_pair` directly for every pair — including
/// which pairs come back `None` (not overlapping), not just the
/// overlapping ones.
#[test]
fn dispatch_matches_direct_test_pair() {
    // 0 and 1 overlap (spheres of radius 0.5 centered 0.6 apart); 0
    // and 2 don't (centered 10 apart).
    let bodies = vec![sphere_at(0.0), sphere_at(0.6), sphere_at(10.0)];
    let pairs = vec![(0usize, 1usize), (0, 2), (1, 2)];

    let narrow_phase = NarrowPhase::<FloatFlavor>::new();
    let expected: Vec<_> = pairs
        .iter()
        .map(|&(a, b)| narrow_phase.test_pair(&bodies, a, b))
        .collect();

    let context = ComputeContext::new();
    let kernel = NarrowPhaseTestPairKernel::new(bodies, pairs.clone());
    kernel.dispatch(&context, DispatchSize::linear(pairs.len() as u32));

    let got = kernel.results();
    assert_eq!(got.len(), expected.len());
    assert!(got[0].is_some(), "0-1 should overlap");
    assert!(got[1].is_none(), "0-2 should not overlap");
    assert!(got[2].is_none(), "1-2 should not overlap");
    for (g, e) in got.iter().zip(expected.iter()) {
        match (g, e) {
            (Some(gc), Some(ec)) => {
                assert_eq!(gc.a, ec.a);
                assert_eq!(gc.b, ec.b);
                assert_eq!(gc.normal, ec.normal);
                assert_eq!(gc.penetration, ec.penetration);
                assert_eq!(gc.point, ec.point);
            }
            (None, None) => {}
            _ => panic!("mismatch: got {g:?}, expected {e:?}"),
        }
    }
}

/// Exercises the CPU `parallel_for` backend's actual parallel path
/// (above `ComputeContext`'s default 1024-item threshold).
#[test]
fn large_batch_matches_direct_test_pair() {
    // A chain of spheres 0.4 apart (radius 0.5, so every consecutive
    // pair overlaps) — enough candidate pairs to cross the parallel
    // threshold, with a real, non-trivial overlap pattern.
    let bodies: Vec<_> = (0..1500).map(|i| sphere_at(i as f32 * 0.4)).collect();
    let pairs: Vec<_> = (0..bodies.len() - 1).map(|i| (i, i + 1)).collect();

    let narrow_phase = NarrowPhase::<FloatFlavor>::new();
    let expected: Vec<_> = pairs
        .iter()
        .map(|&(a, b)| narrow_phase.test_pair(&bodies, a, b))
        .collect();

    let context = ComputeContext::new();
    let kernel = NarrowPhaseTestPairKernel::new(bodies, pairs.clone());
    kernel.dispatch(&context, DispatchSize::linear(pairs.len() as u32));

    let got = kernel.results();
    for (g, e) in got.iter().zip(expected.iter()) {
        assert_eq!(g.is_some(), e.is_some());
        if let (Some(gc), Some(ec)) = (g, e) {
            assert_eq!(gc.point, ec.point);
            assert_eq!(gc.penetration, ec.penetration);
        }
    }
}

fn cuboid_at(x: f32, y: f32, half: f32) -> RigidBody<FloatFlavor> {
    RigidBody {
        frame: Motor3::translation(Vec3::new(x, y, 0.0)),
        velocity: Vec3::ZERO,
        angular_velocity: meridian_gac_core::Bivector3::ZERO,
        mass: 1.0,
        shape: ColliderShape::Cuboid {
            half_extents: Vec3::new(half, half, half),
        },
    }
}

/// A wide, flat floor and a small box resting flush on top — the
/// same shape `physics-core::float`'s own
/// `full_step_cuboid_settles_on_static_cuboid_floor_without_sinking_through`
/// test uses, chosen specifically because the box's face sits well
/// inside the floor's much larger face, guaranteeing `face_manifold`
/// keeps all 4 corners (a real multi-point manifold, not a
/// degenerate edge/corner contact).
fn floor_and_box() -> Vec<RigidBody<FloatFlavor>> {
    vec![
        RigidBody {
            shape: ColliderShape::Cuboid {
                half_extents: Vec3::new(10.0, 5.0, 10.0),
            },
            ..cuboid_at(0.0, -5.0, 0.0)
        },
        RigidBody {
            shape: ColliderShape::Cuboid {
                half_extents: Vec3::new(0.5, 0.5, 0.5),
            },
            ..cuboid_at(0.0, 0.45, 0.0)
        },
    ]
}

/// `GenerateContactsKernel`'s batched (one-dispatch-per-pair) output,
/// flattened back in pair order, must be identical to calling
/// `NarrowPhase::generate_contacts` once directly on the whole
/// `pairs` slice — including the multi-point box-box manifold case
/// this kernel exists for (unlike `NarrowPhaseTestPairKernel`, which
/// only ever sees `test_pair`'s single-point collapse).
#[test]
fn dispatch_matches_direct_generate_contacts() {
    let bodies = floor_and_box();
    let pairs = vec![(0usize, 1usize)];

    let narrow_phase = NarrowPhase::<FloatFlavor>::new();
    let expected = narrow_phase.generate_contacts(&bodies, &pairs);
    assert!(
        expected.len() > 1,
        "test setup should produce a multi-point manifold, got {}",
        expected.len()
    );

    let context = ComputeContext::new();
    let kernel = GenerateContactsKernel::new(bodies, pairs);
    kernel.dispatch(&context, DispatchSize::linear(kernel.pairs.len() as u32));

    let got = kernel.results();
    assert_eq!(got.len(), expected.len());
    for (g, e) in got.iter().zip(expected.iter()) {
        assert_eq!(g.a, e.a);
        assert_eq!(g.b, e.b);
        assert_eq!(g.normal, e.normal);
        assert_eq!(g.penetration, e.penetration);
        assert_eq!(g.point, e.point);
        assert_eq!(g.suppress_angular_response, e.suppress_angular_response);
    }
}

/// A mix of overlapping and non-overlapping pairs, and a
/// non-cuboid-cuboid pair (single-point `test_pair` fallback inside
/// `generate_contacts` itself) alongside a multi-point one — the
/// kernel must reproduce both shapes correctly in the same batch.
#[test]
fn dispatch_matches_direct_generate_contacts_mixed_pairs() {
    let mut bodies = floor_and_box(); // 0: floor, 1: box resting on floor (multi-point)
    bodies.push(sphere_at(0.6)); // 2: elsewhere, doesn't overlap 0 or 1
    bodies.push(cuboid_at(100.0, 0.0, 1.0)); // 3: far away, no overlap with 0
    let pairs = vec![(0usize, 1usize), (0, 3), (1, 2)];

    let narrow_phase = NarrowPhase::<FloatFlavor>::new();
    let expected = narrow_phase.generate_contacts(&bodies, &pairs);

    let context = ComputeContext::new();
    let kernel = GenerateContactsKernel::new(bodies, pairs);
    kernel.dispatch(&context, DispatchSize::linear(kernel.pairs.len() as u32));

    let got = kernel.results();
    assert_eq!(got.len(), expected.len());
    for (g, e) in got.iter().zip(expected.iter()) {
        assert_eq!(g.a, e.a);
        assert_eq!(g.b, e.b);
        assert_eq!(g.point, e.point);
        assert_eq!(g.penetration, e.penetration);
    }
}
