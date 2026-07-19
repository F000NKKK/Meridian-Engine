//! Batch execution kernels for `gac-core`'s `Motor3` transforms — the adapter between pure geometric algebra and `compute-runtime`'s dispatch interface. See docs/adr/007-batch-transforms-via-compute.md.
//!
//! `ComputeKernel::dispatch` takes `&self`, not `&mut self` (multiple
//! dispatch invocations can run concurrently against the same context),
//! so both kernels below hold their output in a `Mutex<Vec<Motor3>>`:
//! indexed writes under the lock, one lock acquisition per work item.
//! That's real per-item lock contention, not free — an acceptable first
//! version (correct, easy to verify by test) rather than the faster but
//! `unsafe`-requiring "each thread gets a disjoint `&mut` slice of the
//! output" version, matching this workspace's established policy of
//! safety-first with the optimization explicitly deferred (see
//! `meridian-task-core`'s shared-ready-queue-not-work-stealing-deques
//! note for the same tradeoff made elsewhere).

use std::sync::Mutex;

use meridian_compute_runtime::{ComputeContext, ComputeKernel, DispatchSize};
use meridian_gac_core::Motor3;

/// Batch-composes a shared `parent` motor with many `locals` — the
/// `world[i] = locals[i].compose(parent)` step of parent/child transform
/// propagation, done as one dispatch instead of once per entity. This is
/// the entry point `ecs-core`'s transform propagation, `physics-core`, and
/// `graphics-core` use for large batches; single-transform math (`motor *
/// local`) stays a direct `gac-core` call and never goes through this
/// crate.
#[derive(Debug)]
pub struct MotorTransformKernel {
    pub parent: Motor3,
    pub locals: Vec<Motor3>,
    results: Mutex<Vec<Motor3>>,
}

impl MotorTransformKernel {
    pub fn new(parent: Motor3, locals: Vec<Motor3>) -> Self {
        let results = Mutex::new(vec![Motor3::identity(); locals.len()]);
        Self {
            parent,
            locals,
            results,
        }
    }

    /// The `world[i] = locals[i].compose(parent)` results from the most
    /// recent dispatch. All-identity if `dispatch` hasn't run yet.
    pub fn results(&self) -> Vec<Motor3> {
        self.results.lock().unwrap().clone()
    }
}

impl ComputeKernel for MotorTransformKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
        let count = size.total().min(self.locals.len());
        context.parallel_for(count, |i| {
            let world = self.locals[i].compose(self.parent);
            self.results.lock().unwrap()[i] = world;
        });
    }
}

/// Batch-composes independent `(child, parent)` pairs — `results[i] =
/// pairs[i].0.compose(pairs[i].1)` — as one dispatch. Unlike
/// [`MotorTransformKernel`], every pair can have a different parent; use
/// this when a batch doesn't share one common parent motor.
#[derive(Debug)]
pub struct MotorComposeKernel {
    pub pairs: Vec<(Motor3, Motor3)>,
    results: Mutex<Vec<Motor3>>,
}

impl MotorComposeKernel {
    pub fn new(pairs: Vec<(Motor3, Motor3)>) -> Self {
        let results = Mutex::new(vec![Motor3::identity(); pairs.len()]);
        Self { pairs, results }
    }

    pub fn results(&self) -> Vec<Motor3> {
        self.results.lock().unwrap().clone()
    }
}

impl ComputeKernel for MotorComposeKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
        let count = size.total().min(self.pairs.len());
        context.parallel_for(count, |i| {
            let (child, parent) = self.pairs[i];
            self.results.lock().unwrap()[i] = child.compose(parent);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_compute_runtime::ComputeScheduler;
    use meridian_gac_core::{Rotor, Vec3};
    use std::f32::consts::PI;

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
        // Force the parallel path (default threshold is 1024; 2000 items
        // clears it, but be explicit so this test doesn't silently start
        // exercising only the sequential path if the default changes).
        let scheduler = ComputeScheduler::with_parallel_threshold(1);
        scheduler.run(&kernel, DispatchSize::linear(locals.len() as u32));

        let results = kernel.results();
        assert_eq!(results.len(), locals.len());

        let p = Vec3::new(1.0, 0.0, 0.0);
        for (i, (&local, &world)) in locals.iter().zip(results.iter()).enumerate() {
            let expected = local.compose(parent);
            assert!(approx_eq(world, expected, p), "mismatch at index {i}");
            // Cross-check against the same "rotate/translate child, then
            // parent" property the roadmap's parent/child hierarchy
            // milestone validated directly in gac-core.
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
        let scheduler = ComputeScheduler::with_parallel_threshold(1);
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
        let scheduler = ComputeScheduler::new();
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
}
