//! Batch execution kernels for `gac-core`'s `Motor3` transforms — the adapter between pure geometric algebra and `compute-runtime`'s dispatch interface. See docs/adr/007-batch-transforms-via-compute.md.
//!
//! [`fixed_wgsl`] is a second, independent kind of GPU work this crate
//! hosts: `Fixed`-point (Q16.16) arithmetic running as real WGSL compute
//! shaders, bit-exact against the CPU `Fixed` implementation — phase 1 of
//! deterministic GPU physics (see that module's own doc comment for
//! scope and the `docs/roadmap.md` GPU-determinism entry for the larger
//! plan).
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

//! [`FixedMotorTransformKernel`]/[`FixedMotorComposeKernel`] are the same
//! two kernels built on `gac-core::fixed_ga::FixedMotor3` instead —
//! deterministic, for `physics-core::fixed`-style consumers that need a
//! batch path too. CPU vs. GPU dispatch is a *setting* a caller picks,
//! not a type-level restriction this crate imposes (see
//! `gac-core`'s crate-root doc comment): `compute-runtime` has no GPU
//! backend implemented yet (see docs/roadmap.md), so both kernels here
//! only exercise the CPU dispatch path today. A future GPU backend could
//! in principle run `Fixed` kernels too, at the cost of `i64` emulation
//! and losing the bit-exact determinism guarantee to GPU execution-order
//! nondeterminism (see docs/adr/008-fixed-point-determinism.md) — a
//! tradeoff for that caller to accept knowingly when it configures a GPU
//! `compute-driver` backend, not something forbidden by these kernels.

use std::sync::Mutex;

pub mod fixed_wgsl;

use meridian_compute_runtime::{ComputeContext, ComputeKernel, DispatchSize};
use meridian_gac_core::Motor3;
use meridian_gac_core::fixed_ga::FixedMotor3;

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

/// Mirrors [`MotorTransformKernel`], built on
/// `gac-core::fixed_ga::FixedMotor3`. See the module doc for why this
/// (and [`FixedMotorComposeKernel`]) are CPU-dispatch only.
#[derive(Debug)]
pub struct FixedMotorTransformKernel {
    pub parent: FixedMotor3,
    pub locals: Vec<FixedMotor3>,
    results: Mutex<Vec<FixedMotor3>>,
}

impl FixedMotorTransformKernel {
    pub fn new(parent: FixedMotor3, locals: Vec<FixedMotor3>) -> Self {
        let results = Mutex::new(vec![FixedMotor3::identity(); locals.len()]);
        Self {
            parent,
            locals,
            results,
        }
    }

    pub fn results(&self) -> Vec<FixedMotor3> {
        self.results.lock().unwrap().clone()
    }
}

impl ComputeKernel for FixedMotorTransformKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
        let count = size.total().min(self.locals.len());
        context.parallel_for(count, |i| {
            let world = self.locals[i].compose(self.parent);
            self.results.lock().unwrap()[i] = world;
        });
    }
}

/// Mirrors [`MotorComposeKernel`], built on
/// `gac-core::fixed_ga::FixedMotor3`.
#[derive(Debug)]
pub struct FixedMotorComposeKernel {
    pub pairs: Vec<(FixedMotor3, FixedMotor3)>,
    results: Mutex<Vec<FixedMotor3>>,
}

impl FixedMotorComposeKernel {
    pub fn new(pairs: Vec<(FixedMotor3, FixedMotor3)>) -> Self {
        let results = Mutex::new(vec![FixedMotor3::identity(); pairs.len()]);
        Self { pairs, results }
    }

    pub fn results(&self) -> Vec<FixedMotor3> {
        self.results.lock().unwrap().clone()
    }
}

impl ComputeKernel for FixedMotorComposeKernel {
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
    use meridian_gac_core::fixed_ga::{FixedRotor, FixedVec3};
    use meridian_gac_core::{Rotor, Vec3};
    use meridian_numeric_core::Fixed;
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
        let scheduler = ComputeScheduler::with_parallel_threshold(1);
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
        let scheduler = ComputeScheduler::with_parallel_threshold(1);
        scheduler.run(&kernel, DispatchSize::linear(pairs.len() as u32));

        let results = kernel.results();
        let p = FixedVec3::ZERO;
        for (i, (&(child, parent), &world)) in pairs.iter().zip(results.iter()).enumerate() {
            let expected = child.compose(parent);
            assert!(fixed_approx_eq(world, expected, p), "mismatch at index {i}");
        }
    }
}
