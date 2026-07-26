//! Batch [`Integrator::step`](meridian_physics_core::generic::Integrator)
//! for many independent [`RigidBody`]s through `compute-runtime`, the
//! rigid-body half of docs/roadmap.md's "batching through compute-runtime
//! is additive later" follow-up (`Integrator`/`BroadPhase`/`NarrowPhase`/
//! `ConstraintSolver` were, until now, only ever called in a plain CPU
//! loop). [`RigidBodyIntegratorKernel`] is [`meridian_gac_compute::MotorTransformKernel`]'s
//! shape applied to `Integrator::step`: one dispatch instead of a
//! `for` loop, correct today via [`ComputeContext::parallel_for`]'s CPU
//! backend — see that kernel's own doc comment for why a `Mutex<Vec<_>>`
//! output, and `compute-runtime`'s module doc for the CPU/GPU dispatch
//! split this crate's `float`/`fixed` soft-body kernels already exercise.
//!
//! **Generic over `F: GaFlavor`, not float/fixed-split**, unlike
//! [`crate::float`]/[`crate::fixed`]: `Integrator::step` has no
//! GPU-dispatch constraint of its own — it's a per-body loop of GA
//! composition/exponential-map calls, the same reason `physics-core`'s
//! own `Integrator` is written once and generic (see that crate's module
//! doc and CLAUDE.md's "Float/Fixed branching" rule). Splitting this
//! kernel into two copies to match `float.rs`/`fixed.rs`'s naming
//! convention would be exactly the disclosed-duplication CLAUDE.md
//! rejects absent a real constraint forcing it.

use std::sync::Mutex;

use meridian_compute_runtime::{ComputeContext, ComputeKernel, DispatchSize};
use meridian_gac_core::generic::GaFlavor;
use meridian_physics_core::generic::{Integrator, RigidBody};

/// Batch-integrates `bodies` by one timestep `dt`, sharing one
/// [`Integrator`] (gravity vector) across the whole batch — the common
/// case, since a scene's bodies almost always share one gravity source.
/// [`RigidBodyIntegratorKernel::results`] holds `Integrator::step`'s
/// output per body, in the same order as the input; static bodies
/// (`mass <= 0`) come back unchanged, matching `Integrator::step`'s own
/// "untouched" behavior for them.
#[derive(Debug)]
pub struct RigidBodyIntegratorKernel<F: GaFlavor> {
    pub integrator: Integrator<F>,
    pub bodies: Vec<RigidBody<F>>,
    pub dt: F::Scalar,
    results: Mutex<Vec<RigidBody<F>>>,
}

impl<F: GaFlavor> RigidBodyIntegratorKernel<F> {
    pub fn new(integrator: Integrator<F>, bodies: Vec<RigidBody<F>>, dt: F::Scalar) -> Self {
        let results = Mutex::new(bodies.clone());
        Self {
            integrator,
            bodies,
            dt,
            results,
        }
    }

    /// The integrated bodies from the most recent [`ComputeKernel::dispatch`]
    /// call — an unintegrated copy of [`bodies`](Self::bodies) if
    /// `dispatch` hasn't run yet.
    pub fn results(&self) -> Vec<RigidBody<F>> {
        self.results.lock().unwrap().clone()
    }
}

impl<F: GaFlavor> ComputeKernel for RigidBodyIntegratorKernel<F> {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
        let count = size.total().min(self.bodies.len());
        context.parallel_for(count, |i| {
            // A one-body slice reuses `Integrator::step`'s exact logic
            // (including its own `inverse_mass() <= ZERO` static-body
            // skip) rather than restating it here — same reason
            // `MotorTransformKernel` calls straight into `gac-core`'s
            // `compose` instead of reimplementing it.
            let mut body = self.bodies[i];
            self.integrator.step(std::slice::from_mut(&mut body), self.dt);
            self.results.lock().unwrap()[i] = body;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_gac_core::float_ga::FloatFlavor;
    use meridian_gac_core::{Motor3, Vec3};
    use meridian_physics_core::generic::ColliderShape;

    fn falling_body(mass: f32) -> RigidBody<FloatFlavor> {
        RigidBody {
            frame: Motor3::identity(),
            velocity: Vec3::ZERO,
            angular_velocity: meridian_gac_core::Bivector3::ZERO,
            mass,
            shape: ColliderShape::Sphere { radius: 0.5 },
        }
    }

    /// The kernel's dispatch, run through `compute-runtime`'s CPU
    /// `parallel_for`, must match calling `Integrator::step` directly on
    /// the same bodies — the whole point of this kernel is "same
    /// algorithm, different dispatch mechanism," not a reimplementation.
    #[test]
    fn dispatch_matches_direct_integrator_step() {
        let integrator = Integrator::<FloatFlavor>::default();
        let bodies: Vec<_> = (0..8).map(|i| falling_body(1.0 + i as f32)).collect();
        let dt = 1.0 / 60.0;

        let mut expected = bodies.clone();
        integrator.step(&mut expected, dt);

        let context = ComputeContext::new();
        let kernel = RigidBodyIntegratorKernel::new(integrator, bodies, dt);
        context.run(&kernel, DispatchSize::linear(kernel.bodies.len() as u32));

        let got = kernel.results();
        assert_eq!(got.len(), expected.len());
        for (g, e) in got.iter().zip(expected.iter()) {
            assert_eq!(g.frame, e.frame);
            assert_eq!(g.velocity, e.velocity);
            assert_eq!(g.angular_velocity, e.angular_velocity);
        }
    }

    /// A static body (`mass <= 0`) must come back completely untouched —
    /// `Integrator::step`'s own skip, which this kernel must not bypass
    /// by, say, integrating it inside a one-body slice differently.
    #[test]
    fn static_bodies_are_untouched() {
        let integrator = Integrator::<FloatFlavor>::default();
        let mut static_body = falling_body(0.0);
        static_body.velocity = Vec3::new(1.0, 2.0, 3.0);
        let before = static_body;

        let context = ComputeContext::new();
        let kernel = RigidBodyIntegratorKernel::new(integrator, vec![static_body], 1.0 / 60.0);
        context.run(&kernel, DispatchSize::linear(1));

        let got = kernel.results();
        assert_eq!(got[0].frame, before.frame);
        assert_eq!(got[0].velocity, before.velocity);
    }

    /// Exercises the CPU `parallel_for` backend's actual parallel path
    /// (above `ComputeContext`'s default 1024-item threshold), not just
    /// the small-batch sequential fallback every other test here hits.
    #[test]
    fn large_batch_matches_direct_integrator_step() {
        let integrator = Integrator::<FloatFlavor>::default();
        let bodies: Vec<_> = (0..2000).map(|i| falling_body(1.0 + (i % 5) as f32)).collect();
        let dt = 1.0 / 60.0;

        let mut expected = bodies.clone();
        integrator.step(&mut expected, dt);

        let context = ComputeContext::new();
        let kernel = RigidBodyIntegratorKernel::new(integrator, bodies, dt);
        context.run(&kernel, DispatchSize::linear(kernel.bodies.len() as u32));

        let got = kernel.results();
        for (g, e) in got.iter().zip(expected.iter()) {
            assert_eq!(g.frame, e.frame);
            assert_eq!(g.velocity, e.velocity);
        }
    }
}
