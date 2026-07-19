//! An opt-in, bit-reproducible alternative to [`crate::RigidBody`]'s
//! default `f32` simulation, for lockstep networking and replay (see
//! `meridian_numeric_core::Fixed`'s doc comment for what that buys and
//! why plain `f32` can't). Everything here mirrors its `f32` counterpart
//! one-for-one (`DeterministicBody` ~ `RigidBody`,
//! `DeterministicIntegrator` ~ `Integrator`, ...) so the two stay easy to
//! compare — this is the same disclosed-duplication approach
//! `gac-core::fixed_ga` takes relative to `gac-core::float_ga`.
//!
//! Scope for this pass: **sphere colliders only**. `Cuboid`/SAT was not
//! ported to fixed-point — that's a large, intricate piece of code and
//! porting it hastily risks subtle bugs in exactly the code whose entire
//! purpose is trustworthy reproducibility. Tracked as explicit follow-up
//! work, not silently dropped.
//!
//! This is a genuinely separate, parallel path — nothing here changes
//! how [`crate::RigidBody`]/[`crate::Integrator`]/etc. behave, and
//! nothing in the rest of the engine has to know a simulation is running
//! deterministically unless it explicitly opts in by constructing
//! [`DeterministicBody`]s instead of [`crate::RigidBody`]s.
//! [`DeterministicBody::frame_f32`] converts the pose to
//! `gac-core::Motor3` for handoff to rendering/ECS/audio, which stay
//! entirely in `f32` regardless of which physics path produced the pose.
//!
//! Geometry (`FixedAabb`, `FixedShape`, ...) comes from
//! `gac-core::fixed_ga`, not reinvented here: this crate's own `f32`
//! pipeline builds its broad-phase `Aabb` from `gac-core::Aabb`/`Obb`
//! (see [`crate::aabb_of`]), and the deterministic pipeline follows the
//! exact same pattern with the `Fixed` equivalents — geometric
//! primitives are `gac-core`'s responsibility regardless of which scalar
//! flavor they're built on, so any other crate that needs
//! CPU-deterministic geometry later (not just this module) can reuse
//! them too.

use meridian_gac_core::fixed_ga::{FixedAabb, FixedBivector3, FixedMotor3, FixedVec3};
use meridian_gac_core::float_ga::Motor3;
use meridian_numeric_core::Fixed;

/// Mirrors [`crate::ColliderShape`] — sphere only for now, see the module
/// doc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeterministicShape {
    Sphere { radius: Fixed },
}

impl Default for DeterministicShape {
    fn default() -> Self {
        DeterministicShape::Sphere {
            radius: Fixed::from_num(0.5),
        }
    }
}

/// Mirrors [`crate::RigidBody`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DeterministicBody {
    pub frame: FixedMotor3,
    pub velocity: FixedVec3,
    pub angular_velocity: FixedBivector3,
    pub mass: Fixed,
    pub shape: DeterministicShape,
}

impl DeterministicBody {
    pub fn inverse_mass(&self) -> Fixed {
        if self.mass > Fixed::ZERO {
            Fixed::ONE / self.mass
        } else {
            Fixed::ZERO
        }
    }

    /// Mirrors [`crate::RigidBody::moment_of_inertia`] — see that
    /// method's doc comment for the same solid-sphere formula.
    pub fn moment_of_inertia(&self) -> Fixed {
        match self.shape {
            DeterministicShape::Sphere { radius } => {
                Fixed::from_num(0.4) * self.mass * radius * radius
            }
        }
    }

    pub fn inverse_inertia(&self) -> Fixed {
        let i = self.moment_of_inertia();
        if i > Fixed::ZERO {
            Fixed::ONE / i
        } else {
            Fixed::ZERO
        }
    }

    pub fn position(&self) -> FixedVec3 {
        self.frame.transform_point(FixedVec3::ZERO)
    }

    /// Converts this body's pose to `gac-core::Motor3` (`f32`) for handoff
    /// to rendering/ECS/audio — `gac-core::fixed_ga::FixedMotor3::to_float_lossy`,
    /// a deliberate, named precision-changing cast (see
    /// docs/gac-design.md's "float_ga/fixed_ga interop" section), not a
    /// re-derivation through rotation/translation.
    pub fn frame_f32(&self) -> Motor3 {
        self.frame.to_float_lossy()
    }

    /// This body's collider as a world-space [`FixedAabb`] — mirrors
    /// [`crate::RigidBody::as_obb`]'s role in the `f32` pipeline (there
    /// building an `Obb`; here a sphere already has its bound directly).
    fn aabb(&self) -> FixedAabb {
        match self.shape {
            DeterministicShape::Sphere { radius } => {
                FixedAabb::from_sphere(self.position(), radius)
            }
        }
    }
}

/// Mirrors [`crate::Contact`].
#[derive(Debug, Clone, Copy, Default)]
pub struct DeterministicContact {
    pub a: usize,
    pub b: usize,
    pub normal: FixedVec3,
    pub penetration: Fixed,
    pub point: FixedVec3,
}

/// Mirrors [`crate::BroadPhase`]: naive O(n²) `FixedAabb` sweep, built
/// from `gac-core::fixed_ga::FixedAabb` the same way `crate::BroadPhase`
/// builds its `Aabb`s from `gac-core::Aabb`, not a hand-rolled overlap
/// test local to this module.
#[derive(Debug, Default)]
pub struct DeterministicBroadPhase {
    pairs: Vec<(usize, usize)>,
}

impl DeterministicBroadPhase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn find_candidate_pairs(&mut self, bodies: &[DeterministicBody]) -> &[(usize, usize)] {
        self.pairs.clear();
        let aabbs: Vec<FixedAabb> = bodies.iter().map(DeterministicBody::aabb).collect();
        for i in 0..bodies.len() {
            for j in (i + 1)..bodies.len() {
                if aabbs[i].overlaps(&aabbs[j]) {
                    self.pairs.push((i, j));
                }
            }
        }
        &self.pairs
    }
}

/// Mirrors [`crate::NarrowPhase`] — sphere-sphere only, see the module doc.
#[derive(Debug, Default)]
pub struct DeterministicNarrowPhase;

impl DeterministicNarrowPhase {
    pub fn new() -> Self {
        Self
    }

    pub fn test_pair(
        &self,
        bodies: &[DeterministicBody],
        a: usize,
        b: usize,
    ) -> Option<DeterministicContact> {
        let DeterministicShape::Sphere { radius: ra } = bodies[a].shape;
        let DeterministicShape::Sphere { radius: rb } = bodies[b].shape;

        let pa = bodies[a].position();
        let pb = bodies[b].position();
        let delta = pb - pa;
        let dist = delta.length();
        let combined = ra + rb;

        if dist >= combined {
            return None;
        }
        let normal = if dist > Fixed::from_bits(4) {
            delta * (Fixed::ONE / dist)
        } else {
            FixedVec3::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO)
        };
        let point = pa + normal * ra;
        Some(DeterministicContact {
            a,
            b,
            normal,
            penetration: combined - dist,
            point,
        })
    }

    pub fn generate_contacts(
        &self,
        bodies: &[DeterministicBody],
        candidate_pairs: &[(usize, usize)],
    ) -> Vec<DeterministicContact> {
        candidate_pairs
            .iter()
            .filter_map(|&(a, b)| self.test_pair(bodies, a, b))
            .collect()
    }
}

/// Mirrors [`crate::ConstraintSolver`].
#[derive(Debug, Clone, Copy)]
pub struct DeterministicConstraintSolver {
    pub restitution: Fixed,
}

impl DeterministicConstraintSolver {
    pub fn new(restitution: Fixed) -> Self {
        Self { restitution }
    }

    pub fn resolve(&self, bodies: &mut [DeterministicBody], contact: &DeterministicContact) {
        let inv_mass_a = bodies[contact.a].inverse_mass();
        let inv_mass_b = bodies[contact.b].inverse_mass();
        let total_inv_mass = inv_mass_a + inv_mass_b;
        if total_inv_mass <= Fixed::ZERO {
            return;
        }

        let relative_velocity = bodies[contact.b].velocity - bodies[contact.a].velocity;
        let velocity_along_normal = relative_velocity.dot(contact.normal);

        if velocity_along_normal < Fixed::ZERO {
            let j = -(Fixed::ONE + self.restitution) * velocity_along_normal / total_inv_mass;
            let impulse = contact.normal * j;
            bodies[contact.a].velocity = bodies[contact.a].velocity - impulse * inv_mass_a;
            bodies[contact.b].velocity = bodies[contact.b].velocity + impulse * inv_mass_b;

            let offset_a = contact.point - bodies[contact.a].position();
            let offset_b = contact.point - bodies[contact.b].position();
            let torque_a = FixedBivector3::wedge(offset_a, impulse);
            let torque_b = FixedBivector3::wedge(offset_b, impulse);
            let inv_inertia_a = bodies[contact.a].inverse_inertia();
            let inv_inertia_b = bodies[contact.b].inverse_inertia();
            bodies[contact.a].angular_velocity =
                bodies[contact.a].angular_velocity - torque_a * inv_inertia_a;
            bodies[contact.b].angular_velocity =
                bodies[contact.b].angular_velocity + torque_b * inv_inertia_b;
        }

        let correction_percent = Fixed::from_num(0.8);
        let slop = Fixed::from_num(0.01);
        let correction_mag =
            (contact.penetration - slop).max(Fixed::ZERO) / total_inv_mass * correction_percent;
        let correction = contact.normal * correction_mag;
        bodies[contact.a].frame = bodies[contact.a]
            .frame
            .compose(FixedMotor3::translation(correction * -inv_mass_a));
        bodies[contact.b].frame = bodies[contact.b]
            .frame
            .compose(FixedMotor3::translation(correction * inv_mass_b));
    }
}

/// Mirrors [`crate::Integrator`].
#[derive(Debug, Clone, Copy)]
pub struct DeterministicIntegrator {
    pub gravity: FixedVec3,
}

impl Default for DeterministicIntegrator {
    fn default() -> Self {
        Self {
            gravity: FixedVec3::new(Fixed::ZERO, Fixed::from_num(-9.81), Fixed::ZERO),
        }
    }
}

impl DeterministicIntegrator {
    pub fn new(gravity: FixedVec3) -> Self {
        Self { gravity }
    }

    pub fn step(&self, bodies: &mut [DeterministicBody], dt: Fixed) {
        for body in bodies.iter_mut() {
            if body.inverse_mass() <= Fixed::ZERO {
                continue;
            }
            body.velocity = body.velocity + self.gravity * dt;
            let rotor_delta = (body.angular_velocity * dt).exp();
            let linear_delta = body.velocity * dt;
            let motor_delta = FixedMotor3::from_rotation_translation(rotor_delta, linear_delta);
            body.frame = body.frame.compose(motor_delta);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sphere(
        position: FixedVec3,
        velocity: FixedVec3,
        mass: f64,
        radius: f64,
    ) -> DeterministicBody {
        DeterministicBody {
            frame: FixedMotor3::translation(position),
            velocity,
            mass: Fixed::from_num(mass),
            shape: DeterministicShape::Sphere {
                radius: Fixed::from_num(radius),
            },
            ..Default::default()
        }
    }

    fn fv3(x: f64, y: f64, z: f64) -> FixedVec3 {
        FixedVec3::new(Fixed::from_num(x), Fixed::from_num(y), Fixed::from_num(z))
    }

    #[test]
    fn broad_phase_finds_only_overlapping_pairs() {
        let bodies = vec![
            sphere(FixedVec3::ZERO, FixedVec3::ZERO, 1.0, 1.0),
            sphere(fv3(1.5, 0.0, 0.0), FixedVec3::ZERO, 1.0, 1.0),
            sphere(fv3(100.0, 0.0, 0.0), FixedVec3::ZERO, 1.0, 1.0),
        ];
        let mut broad = DeterministicBroadPhase::new();
        let pairs = broad.find_candidate_pairs(&bodies);
        assert_eq!(pairs, &[(0, 1)]);
    }

    #[test]
    fn narrow_phase_reports_normal_pointing_from_a_to_b_and_correct_penetration() {
        let bodies = vec![
            sphere(FixedVec3::ZERO, FixedVec3::ZERO, 1.0, 1.0),
            sphere(fv3(1.5, 0.0, 0.0), FixedVec3::ZERO, 1.0, 1.0),
        ];
        let contact = DeterministicNarrowPhase::new()
            .test_pair(&bodies, 0, 1)
            .unwrap();
        // Fixed division rounds, so this is approximate, not exact equality.
        assert!((contact.normal.x.to_num() - 1.0).abs() < 1e-3);
        assert!(contact.normal.y.to_num().abs() < 1e-3);
        assert!(contact.normal.z.to_num().abs() < 1e-3);
        let penetration = contact.penetration.to_num();
        assert!((penetration - 0.5).abs() < 1e-3);
    }

    #[test]
    fn integrator_applies_gravity_and_moves_dynamic_bodies() {
        let mut bodies = vec![sphere(FixedVec3::ZERO, FixedVec3::ZERO, 1.0, 0.5)];
        DeterministicIntegrator::default().step(&mut bodies, Fixed::from_num(1.0));
        assert!((bodies[0].velocity.y.to_num() - -9.81).abs() < 1e-2);
        assert!(bodies[0].position().y.to_num() < 0.0);
    }

    #[test]
    fn integrator_never_moves_static_bodies() {
        let mut bodies = vec![sphere(FixedVec3::ZERO, FixedVec3::ZERO, 0.0, 1.0)];
        DeterministicIntegrator::default().step(&mut bodies, Fixed::from_num(1.0));
        assert_eq!(bodies[0].velocity, FixedVec3::ZERO);
        assert_eq!(bodies[0].position(), FixedVec3::ZERO);
    }

    #[test]
    fn full_step_ball_settles_on_static_floor_without_sinking_through() {
        let floor_radius = 50.0;
        let ball_radius = 0.5;
        let mut bodies = vec![
            sphere(
                fv3(0.0, -floor_radius, 0.0),
                FixedVec3::ZERO,
                0.0,
                floor_radius,
            ),
            sphere(fv3(0.0, 3.0, 0.0), FixedVec3::ZERO, 1.0, ball_radius),
        ];

        let integrator = DeterministicIntegrator::default();
        let solver = DeterministicConstraintSolver::new(Fixed::from_num(0.1));
        let mut broad = DeterministicBroadPhase::new();
        let narrow = DeterministicNarrowPhase::new();
        let dt = Fixed::from_num(1.0 / 60.0);

        for _ in 0..600 {
            integrator.step(&mut bodies, dt);
            let pairs = broad.find_candidate_pairs(&bodies).to_vec();
            for contact in narrow.generate_contacts(&bodies, &pairs) {
                solver.resolve(&mut bodies, &contact);
            }
        }

        let resting_height = bodies[1].position().y.to_num();
        assert!(
            (resting_height - ball_radius).abs() < 0.5,
            "ball should settle near the floor surface, got y={resting_height}"
        );
    }

    #[test]
    fn frame_f32_conversion_round_trips_position() {
        let body = sphere(fv3(1.5, -2.5, 3.5), FixedVec3::ZERO, 1.0, 0.5);
        let f32_frame = body.frame_f32();
        let position = f32_frame.transform_point(meridian_gac_core::Vec3::ZERO);
        assert!((position.x - 1.5).abs() < 1e-3);
        assert!((position.y - -2.5).abs() < 1e-3);
        assert!((position.z - 3.5).abs() < 1e-3);
    }

    /// The actual point of this whole module: run the identical scenario
    /// (gravity, a collision with restitution, positional correction —
    /// every piece of the pipeline that matters) through two completely
    /// independent simulations and require the result to be bit-for-bit
    /// identical, not merely "close". `Fixed`'s `PartialEq` compares raw
    /// `i32` bits, so this is an exact equality check, unlike every other
    /// test in this file that (correctly) uses a tolerance.
    ///
    /// This proves reproducibility *within one process*, which is a
    /// necessary but not sufficient condition for the actual goal
    /// (cross-platform/cross-build reproducibility for lockstep
    /// networking) — a single test process can't independently verify
    /// behavior on different hardware. What it does verify: nothing in
    /// this pipeline introduces incidental nondeterminism of its own
    /// (uninitialized memory, hash-map iteration order, thread
    /// scheduling, ...) on top of `Fixed`'s own by-construction
    /// cross-platform guarantee (plain `i32`/`i64` `+`/`-`/`*`/`/`, which
    /// the IEEE-754 float path this module exists to avoid can't promise
    /// — see `meridian_numeric_core::Fixed`'s doc comment).
    #[test]
    fn identical_inputs_produce_bit_identical_output_after_many_steps() {
        fn run() -> Vec<DeterministicBody> {
            let mut bodies = vec![
                sphere(fv3(0.0, -50.0, 0.0), FixedVec3::ZERO, 0.0, 50.0),
                sphere(fv3(0.3, 5.0, -0.2), fv3(0.1, 0.0, 0.05), 1.0, 0.5),
                sphere(fv3(-0.4, 8.0, 0.1), FixedVec3::ZERO, 1.0, 0.5),
            ];
            let integrator = DeterministicIntegrator::default();
            let solver = DeterministicConstraintSolver::new(Fixed::from_num(0.3));
            let mut broad = DeterministicBroadPhase::new();
            let narrow = DeterministicNarrowPhase::new();
            let dt = Fixed::from_num(1.0 / 60.0);

            for _ in 0..300 {
                integrator.step(&mut bodies, dt);
                let pairs = broad.find_candidate_pairs(&bodies).to_vec();
                for contact in narrow.generate_contacts(&bodies, &pairs) {
                    solver.resolve(&mut bodies, &contact);
                }
            }
            bodies
        }

        let run_a = run();
        let run_b = run();
        assert_eq!(
            run_a, run_b,
            "identical deterministic simulations must produce bit-identical state"
        );
    }
}
