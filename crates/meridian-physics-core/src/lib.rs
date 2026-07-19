//! Rigid body dynamics built on the GAC: broad/narrow phase collision, constraint solving and integration.
//!
//! Real, tested physics pipeline: [`BroadPhase`] (naive O(n²) AABB sweep —
//! a spatial hash/BVH is a later optimization once profiling calls for
//! it, same policy as `task-core`'s scheduler), [`NarrowPhase`]
//! (sphere-sphere only so far — the only [`ColliderShape`] variant right
//! now), [`ConstraintSolver`] (impulse-based, linear *and* angular, with
//! positional correction against sinking), and [`Integrator`] (semi-
//! implicit Euler, using `gac-core`'s bivector exponential map for
//! rotation — not a naive "add angle" or a separately-tracked quaternion).
//! No GPU/SIMD dispatch through `compute-runtime` wired in yet — these
//! are correct sequential CPU implementations first; batching them
//! through `compute-runtime` is additive later, not a rewrite (the same
//! kernel logic, called per-pair instead of once).
//!
//! GA is used where the physics actually calls for it, not decoratively:
//! angular velocity and torque are [`Bivector3`] (they live in the Lie
//! algebra of rotations, so(3) — a bivector space, not a vector space, per
//! `gac-core`'s own doc comment on the type), and orientation is
//! integrated via `Bivector3::exp` (a rotor exponential), composed onto
//! `Motor3` — this is what keeps a spinning body's orientation exactly on
//! the unit-rotor manifold frame after frame, the same reason `gac-core`
//! uses motors for `Transform` at all instead of a quaternion+vector pair
//! (see [ADR 001](../../../docs/adr/001-geometric-algebra-as-spatial-model.md)).
//! Linear velocity stays a plain `Vec3` — GA doesn't say vectors are
//! wrong, only that angular quantities specifically are bivectors.

use meridian_gac_core::{Bivector3, Motor3, Vec3};
use meridian_resource_core::ResourceId;

/// Marker type for collider-mesh `ResourceId`s — see
/// docs/adr/006-resource-core-separation.md.
pub struct ColliderMeshMarker;
pub type ColliderMeshHandle = ResourceId<ColliderMeshMarker>;

/// A collision shape. Only `Sphere` for now — the simplest shape to get
/// narrow-phase exactly right; box/capsule/mesh (via
/// [`ColliderMeshHandle`]) are additive later.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColliderShape {
    Sphere { radius: f32 },
}

impl Default for ColliderShape {
    fn default() -> Self {
        ColliderShape::Sphere { radius: 0.5 }
    }
}

/// A simulated rigid body: spatial frame (shared with every other
/// subsystem via the GAC) + linear and angular state. `mass <= 0.0` means
/// static/immovable (infinite mass) — never touched by [`Integrator`] or
/// given any velocity change by [`ConstraintSolver`].
#[derive(Debug, Clone, Copy, Default)]
pub struct RigidBody {
    pub frame: Motor3,
    pub velocity: Vec3,
    pub angular_velocity: Bivector3,
    pub mass: f32,
    pub shape: ColliderShape,
}

impl RigidBody {
    pub fn inverse_mass(&self) -> f32 {
        if self.mass > 0.0 {
            1.0 / self.mass
        } else {
            0.0
        }
    }

    /// Moment of inertia about the body's center of mass. Only the solid-
    /// sphere formula (`(2/5) * m * r²`) exists so far — other shapes need
    /// their own formula once they exist.
    pub fn moment_of_inertia(&self) -> f32 {
        match self.shape {
            ColliderShape::Sphere { radius } => 0.4 * self.mass * radius * radius,
        }
    }

    pub fn inverse_inertia(&self) -> f32 {
        let i = self.moment_of_inertia();
        if i > 0.0 {
            1.0 / i
        } else {
            0.0
        }
    }

    pub fn position(&self) -> Vec3 {
        self.frame.transform_point(Vec3::ZERO)
    }
}

/// An axis-aligned bounding box, used by [`BroadPhase`] to cheaply reject
/// pairs that can't possibly be touching before running exact narrow-phase
/// tests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn from_sphere(center: Vec3, radius: f32) -> Self {
        let r = Vec3::new(radius, radius, radius);
        Self { min: center - r, max: center + r }
    }

    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }
}

fn aabb_of(body: &RigidBody) -> Aabb {
    match body.shape {
        ColliderShape::Sphere { radius } => Aabb::from_sphere(body.position(), radius),
    }
}

/// A broad-phase acceleration structure (BVH, spatial hash, ...). Owned
/// here, not in `physics-driver` — see docs/physics-design.md. Currently a
/// naive O(n²) AABB sweep.
#[derive(Debug, Default)]
pub struct BroadPhase {
    pairs: Vec<(usize, usize)>,
}

impl BroadPhase {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns index pairs into `bodies` whose AABBs overlap — candidates
    /// for narrow-phase, not confirmed contacts.
    pub fn find_candidate_pairs(&mut self, bodies: &[RigidBody]) -> &[(usize, usize)] {
        self.pairs.clear();
        let aabbs: Vec<Aabb> = bodies.iter().map(aabb_of).collect();
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

/// A single narrow-phase contact between two colliders (indices into the
/// same `&[RigidBody]` slice `BroadPhase`/`NarrowPhase` were given).
/// `normal` points from `a` toward `b`; `point` is the contact point in
/// world space, used by [`ConstraintSolver`] to compute torque.
#[derive(Debug, Clone, Copy, Default)]
pub struct Contact {
    pub a: usize,
    pub b: usize,
    pub normal: Vec3,
    pub penetration: f32,
    pub point: Vec3,
}

/// Resolves candidate pairs from [`BroadPhase`] into exact [`Contact`]s.
#[derive(Debug, Default)]
pub struct NarrowPhase;

impl NarrowPhase {
    pub fn new() -> Self {
        Self
    }

    /// Exact sphere-sphere test. `None` if not overlapping.
    pub fn test_pair(&self, bodies: &[RigidBody], a: usize, b: usize) -> Option<Contact> {
        let ColliderShape::Sphere { radius: ra } = bodies[a].shape;
        let ColliderShape::Sphere { radius: rb } = bodies[b].shape;

        let pa = bodies[a].position();
        let pb = bodies[b].position();
        let delta = pb - pa;
        let dist = delta.length();
        let combined = ra + rb;

        if dist >= combined {
            return None;
        }
        // Centers coincide exactly: pick an arbitrary separating axis
        // rather than dividing by a zero-length delta.
        let normal = if dist > 1e-6 { delta * (1.0 / dist) } else { Vec3::X };
        let point = pa + normal * ra;
        Some(Contact { a, b, normal, penetration: combined - dist, point })
    }

    pub fn generate_contacts(&self, bodies: &[RigidBody], candidate_pairs: &[(usize, usize)]) -> Vec<Contact> {
        candidate_pairs.iter().filter_map(|&(a, b)| self.test_pair(bodies, a, b)).collect()
    }
}

/// Resolves contacts into corrective linear *and* angular impulses
/// (conserving momentum) plus a small positional correction so resting
/// contacts don't sink into each other frame over frame.
///
/// The angular part is a simplified, not fully coupled, solver: it
/// applies torque from the same normal impulse computed for the linear
/// response, via `Bivector3::wedge(contact_offset, impulse)`, but doesn't
/// feed the rotational inertia back into the impulse magnitude itself
/// (the full 6-DOF effective-mass formulation does). With sphere-sphere
/// contacts specifically this is provably inert anyway: a sphere pair's
/// contact point always lies on the line between the two centers, so the
/// offset vector is parallel to the impulse and the wedge product is
/// zero — no spurious spin from a dead-center hit, which is physically
/// correct, but it also means angular response has no sphere-only test
/// that isn't "check it's zero." It's wired in now, ready for when a
/// non-spherical shape makes the offset non-parallel to the normal.
#[derive(Debug, Clone, Copy)]
pub struct ConstraintSolver {
    /// 0.0 = perfectly inelastic (bodies stick, no bounce), 1.0 = perfectly
    /// elastic (no kinetic energy lost).
    pub restitution: f32,
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self { restitution: 0.0 }
    }
}

const POSITIONAL_CORRECTION_PERCENT: f32 = 0.8;
const POSITIONAL_CORRECTION_SLOP: f32 = 0.01;

impl ConstraintSolver {
    pub fn new(restitution: f32) -> Self {
        Self { restitution }
    }

    /// Applies an impulse-based resolution for `contact`, mutating only
    /// `bodies[contact.a]` and `bodies[contact.b]`. A no-op if both bodies
    /// are static (infinite mass on both sides — nothing to resolve).
    pub fn resolve(&self, bodies: &mut [RigidBody], contact: &Contact) {
        let inv_mass_a = bodies[contact.a].inverse_mass();
        let inv_mass_b = bodies[contact.b].inverse_mass();
        let total_inv_mass = inv_mass_a + inv_mass_b;
        if total_inv_mass <= 0.0 {
            return;
        }

        let relative_velocity = bodies[contact.b].velocity - bodies[contact.a].velocity;
        let velocity_along_normal = relative_velocity.dot(contact.normal);

        if velocity_along_normal < 0.0 {
            let j = -(1.0 + self.restitution) * velocity_along_normal / total_inv_mass;
            let impulse = contact.normal * j;
            bodies[contact.a].velocity = bodies[contact.a].velocity - impulse * inv_mass_a;
            bodies[contact.b].velocity = bodies[contact.b].velocity + impulse * inv_mass_b;

            // Angular response — see the type's doc comment on why this
            // is a simplified, decoupled solve, and provably zero for
            // sphere-sphere contacts specifically.
            let offset_a = contact.point - bodies[contact.a].position();
            let offset_b = contact.point - bodies[contact.b].position();
            let torque_a = Bivector3::wedge(offset_a, impulse);
            let torque_b = Bivector3::wedge(offset_b, impulse);
            let inv_inertia_a = bodies[contact.a].inverse_inertia();
            let inv_inertia_b = bodies[contact.b].inverse_inertia();
            bodies[contact.a].angular_velocity = bodies[contact.a].angular_velocity - torque_a * inv_inertia_a;
            bodies[contact.b].angular_velocity = bodies[contact.b].angular_velocity + torque_b * inv_inertia_b;
        }

        let correction_mag =
            (contact.penetration - POSITIONAL_CORRECTION_SLOP).max(0.0) / total_inv_mass * POSITIONAL_CORRECTION_PERCENT;
        let correction = contact.normal * correction_mag;
        bodies[contact.a].frame = bodies[contact.a].frame.compose(Motor3::translation(correction * -inv_mass_a));
        bodies[contact.b].frame = bodies[contact.b].frame.compose(Motor3::translation(correction * inv_mass_b));
    }
}

/// Advances rigid body velocity/angular velocity and position/orientation
/// by one timestep (semi-implicit Euler: velocity is updated by gravity
/// first, then position/orientation are updated using the *new* velocity
/// — more stable than explicit Euler for the same cost). Orientation is
/// integrated via `Bivector3::exp` (rotor exponential map), not a naive
/// "add angle" — see the module doc. Static bodies (`mass <= 0.0`) are
/// untouched.
#[derive(Debug, Clone, Copy)]
pub struct Integrator {
    pub gravity: Vec3,
}

impl Default for Integrator {
    fn default() -> Self {
        Self { gravity: Vec3::new(0.0, -9.81, 0.0) }
    }
}

impl Integrator {
    pub fn new(gravity: Vec3) -> Self {
        Self { gravity }
    }

    pub fn step(&self, bodies: &mut [RigidBody], dt: f32) {
        for body in bodies.iter_mut() {
            if body.inverse_mass() <= 0.0 {
                continue;
            }
            body.velocity = body.velocity + self.gravity * dt;
            // No torque source here yet (gravity acts at the center of
            // mass, so it never torques a body), so angular_velocity is
            // untouched by gravity — only ConstraintSolver changes it.
            let rotor_delta = (body.angular_velocity * dt).exp();
            let linear_delta = body.velocity * dt;
            let motor_delta = Motor3::from_rotation_translation(rotor_delta, linear_delta);
            body.frame = body.frame.compose(motor_delta);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    fn sphere(position: Vec3, velocity: Vec3, mass: f32, radius: f32) -> RigidBody {
        RigidBody { frame: Motor3::translation(position), velocity, mass, shape: ColliderShape::Sphere { radius }, ..Default::default() }
    }

    #[test]
    fn aabb_overlap_basic_cases() {
        let a = Aabb::from_sphere(Vec3::ZERO, 1.0);
        let touching = Aabb::from_sphere(Vec3::new(2.0, 0.0, 0.0), 1.0);
        let separate = Aabb::from_sphere(Vec3::new(10.0, 0.0, 0.0), 1.0);
        assert!(a.overlaps(&touching), "boxes sharing a boundary count as overlapping");
        assert!(!a.overlaps(&separate));
    }

    #[test]
    fn broad_phase_finds_only_overlapping_pairs() {
        let bodies = vec![
            sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0),
            sphere(Vec3::new(1.5, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0), // overlaps body 0
            sphere(Vec3::new(100.0, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0), // far away
        ];
        let mut broad = BroadPhase::new();
        let pairs = broad.find_candidate_pairs(&bodies);
        assert_eq!(pairs, &[(0, 1)]);
    }

    #[test]
    fn narrow_phase_reports_normal_pointing_from_a_to_b_and_correct_penetration() {
        let bodies = vec![sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0), sphere(Vec3::new(1.5, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0)];
        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        assert_eq!(contact.normal, Vec3::new(1.0, 0.0, 0.0));
        assert!((contact.penetration - 0.5).abs() < 1e-5, "spheres of radius 1 each, centers 1.5 apart -> 0.5 overlap");
    }

    #[test]
    fn narrow_phase_returns_none_when_not_touching() {
        let bodies = vec![sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0), sphere(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0)];
        assert!(NarrowPhase::new().test_pair(&bodies, 0, 1).is_none());
    }

    #[test]
    fn solver_separates_bodies_moving_toward_each_other() {
        let mut bodies = vec![
            sphere(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 1.0, 1.0),
            sphere(Vec3::new(1.5, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0), 1.0, 1.0),
        ];
        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        ConstraintSolver::new(1.0).resolve(&mut bodies, &contact);

        let relative_velocity_after = bodies[1].velocity - bodies[0].velocity;
        assert!(relative_velocity_after.dot(contact.normal) >= 0.0, "bodies must no longer be closing after resolution");
    }

    #[test]
    fn solver_conserves_momentum_in_elastic_collision() {
        let mut bodies = vec![sphere(Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), 2.0, 1.0), sphere(Vec3::new(1.5, 0.0, 0.0), Vec3::ZERO, 1.0, 1.0)];
        let momentum_before = bodies[0].velocity * bodies[0].mass + bodies[1].velocity * bodies[1].mass;

        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        ConstraintSolver::new(1.0).resolve(&mut bodies, &contact);

        let momentum_after = bodies[0].velocity * bodies[0].mass + bodies[1].velocity * bodies[1].mass;
        assert!((momentum_before - momentum_after).length() < 1e-4, "impulse resolution must conserve momentum: before {momentum_before:?}, after {momentum_after:?}");
    }

    #[test]
    fn solver_does_not_move_a_static_body() {
        let mut bodies = vec![
            sphere(Vec3::ZERO, Vec3::ZERO, 0.0, 1.0), // static: mass = 0
            sphere(Vec3::new(1.5, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0), 1.0, 1.0),
        ];
        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        ConstraintSolver::new(0.0).resolve(&mut bodies, &contact);
        assert_eq!(bodies[0].velocity, Vec3::ZERO, "static body's velocity must never change");
        assert_eq!(bodies[0].position(), Vec3::ZERO, "static body's position must never change");
    }

    #[test]
    fn sphere_sphere_contact_produces_no_spurious_spin() {
        // A sphere pair's contact point is always on the line between the
        // two centers, so the impulse's torque contribution is provably
        // zero — a dead-center hit must not spin either sphere.
        let mut bodies = vec![
            sphere(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 1.0, 1.0),
            sphere(Vec3::new(1.5, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0), 1.0, 1.0),
        ];
        let contact = NarrowPhase::new().test_pair(&bodies, 0, 1).unwrap();
        ConstraintSolver::new(1.0).resolve(&mut bodies, &contact);

        assert_eq!(bodies[0].angular_velocity, Bivector3::ZERO);
        assert_eq!(bodies[1].angular_velocity, Bivector3::ZERO);
    }

    #[test]
    fn integrator_applies_gravity_and_moves_dynamic_bodies() {
        let mut bodies = vec![sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 0.5)];
        Integrator::default().step(&mut bodies, 1.0);
        assert!((bodies[0].velocity.y - (-9.81)).abs() < 1e-4);
        assert!(bodies[0].position().y < 0.0, "body must have fallen");
    }

    #[test]
    fn integrator_never_moves_static_bodies() {
        let mut bodies = vec![sphere(Vec3::ZERO, Vec3::ZERO, 0.0, 1.0)];
        Integrator::default().step(&mut bodies, 1.0);
        assert_eq!(bodies[0].velocity, Vec3::ZERO);
        assert_eq!(bodies[0].position(), Vec3::ZERO);
    }

    #[test]
    fn integrator_rotates_a_spinning_body_via_bivector_exponential() {
        let mut bodies = vec![sphere(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0)];
        bodies[0].angular_velocity = Bivector3::new(0.0, 0.0, PI); // spinning about Z at pi rad/s

        let dt = 1.0 / 120.0;
        for _ in 0..120 {
            Integrator::default().step(&mut bodies, dt);
        }
        // After 1 second at pi rad/s about Z, the body's local +X axis
        // should point along world -X (half turn).
        let local_x_direction = bodies[0].frame.transform_point(Vec3::X) - bodies[0].position();
        assert!((local_x_direction - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-3, "expected a half-turn about Z, got local X direction {local_x_direction:?}");
    }

    #[test]
    fn full_step_ball_settles_on_static_floor_without_sinking_through() {
        // A dynamic ball starts above a large static "floor" sphere and
        // falls under gravity; over many steps it must come to rest near
        // the floor's surface rather than sinking through it or bouncing
        // away forever.
        let floor_radius = 50.0;
        let ball_radius = 0.5;
        let mut bodies = vec![
            sphere(Vec3::new(0.0, -floor_radius, 0.0), Vec3::ZERO, 0.0, floor_radius),
            sphere(Vec3::new(0.0, 3.0, 0.0), Vec3::ZERO, 1.0, ball_radius),
        ];

        let integrator = Integrator::default();
        let solver = ConstraintSolver::new(0.1);
        let mut broad = BroadPhase::new();
        let narrow = NarrowPhase::new();
        let dt = 1.0 / 60.0;

        for _ in 0..600 {
            integrator.step(&mut bodies, dt);
            let pairs = broad.find_candidate_pairs(&bodies).to_vec();
            for contact in narrow.generate_contacts(&bodies, &pairs) {
                solver.resolve(&mut bodies, &contact);
            }
        }

        // Floor center is at y=-floor_radius, so its top surface is at
        // y=0; the ball rests with its center one radius above that.
        let resting_height = bodies[1].position().y;
        let expected = ball_radius;
        assert!((resting_height - expected).abs() < 0.5, "ball should settle near the floor surface, got y={resting_height}, expected near {expected}");
    }
}
