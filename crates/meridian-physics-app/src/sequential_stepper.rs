//! The plain sequential CPU physics step.

use crate::{PhysicsStepper, PhysicsSubsystem, RELAXATION_ITERATIONS};

/// Steps physics with a plain single-threaded loop over `physics-domain`.
///
/// Integrates, then relaxes contacts over `RELAXATION_ITERATIONS`
/// velocity-only passes before a single final positional correction.
/// **Not** one `resolve()` call per contact: a box/pyramid manifold is up
/// to 4 contact points sharing one normal, and a combined
/// velocity-plus-positional pass over all of them pushes the body by the
/// same correction several times per tick, so a settled box bounces and
/// clips through the floor (see `ConstraintSolver::resolve`).
#[derive(Debug, Default, Clone, Copy)]
pub struct SequentialStepper;

impl PhysicsStepper for SequentialStepper {
    fn step(&mut self, physics: &mut PhysicsSubsystem, dt: f32) {
        physics.integrator.step(&mut physics.bodies, dt);

        for _ in 0..RELAXATION_ITERATIONS {
            let pairs = physics
                .broad_phase
                .find_candidate_pairs(&physics.bodies)
                .to_vec();
            for contact in physics
                .narrow_phase
                .generate_contacts(&physics.bodies, &pairs)
            {
                physics
                    .solver
                    .resolve_velocity(&mut physics.bodies, &contact);
            }
        }
        // Positional correction exactly once per tick, against the final
        // (velocity-relaxed) contact set.
        let pairs = physics
            .broad_phase
            .find_candidate_pairs(&physics.bodies)
            .to_vec();
        for contact in physics
            .narrow_phase
            .generate_contacts(&physics.bodies, &pairs)
        {
            physics
                .solver
                .apply_positional_correction(&mut physics.bodies, &contact);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::run_stepper_contract;

    #[test]
    fn sequential_stepper_passes_the_contract() {
        run_stepper_contract(&mut SequentialStepper);
    }
}
