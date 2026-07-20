//! A mass-spring soft-body model — deformable bodies (a "rubber ball"
//! that squashes on impact and recovers), generic over `F: GaFlavor` for
//! the same reason [`crate::generic`]'s rigid-body engine is: nothing
//! here has a GPU-dispatch constraint of its own, so it's written once
//! rather than duplicated per scalar flavor (see CLAUDE.md's "Float/Fixed
//! branching" rule). [`super::float_softbody`]/[`super::fixed_softbody`]
//! alias these types the same way `crate::float`/`crate::fixed` alias the
//! rigid-body engine.
//!
//! [`icosphere_soft_body`] builds a ball from `gac-core::generic::icosphere`
//! (mesh topology is a geometry concern shared by physics/graphics/assets,
//! not this crate's to reinvent — see that function's own doc comment):
//! the mesh's vertices become particles, its edges become structural
//! springs (keep the surface shape), plus one center particle connected
//! to every surface vertex by a spoke spring (keeps the ball from
//! collapsing — without spokes, a pure edge-spring surface has no
//! resistance to being pushed inward, since edge springs only resist
//! *along the surface*, not radially). [`SoftBodyIntegrator`] steps
//! gravity, spring forces (Hooke's law plus a velocity-relative damping
//! term, since undamped springs oscillate forever), and a simple
//! ground-plane collision (push out of penetration, reflect velocity with
//! restitution) — deliberately not reusing [`crate::generic`]'s
//! rigid-body [`crate::generic::ConstraintSolver`]/
//! [`crate::generic::NarrowPhase`], which operate on whole rigid colliders
//! (spheres/cuboids), not individual particles; a soft body's collision
//! response is inherently per-particle.

use meridian_gac_core::generic::{GaFlavor, Plane, ScalarLike, VectorLike, icosphere};

/// A structural or volume-preserving connection between two particles in
/// a [`SoftBody`], by index into its `positions`/`velocities`/
/// `inverse_masses`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring<F: GaFlavor> {
    pub a: usize,
    pub b: usize,
    pub rest_length: F::Scalar,
    /// Hooke's-law spring constant: force magnitude per unit of
    /// (`current_length - rest_length`) stretch/compression.
    pub stiffness: F::Scalar,
    /// Velocity-relative damping coefficient — without this, a spring
    /// oscillates indefinitely (no energy loss); with it, stretch/
    /// compression along the spring's own axis bleeds off over time,
    /// which is what actually makes a squashed ball recover to a
    /// resting shape instead of jiggling forever.
    pub damping: F::Scalar,
}

/// A deformable body: particles (point masses, no orientation — unlike
/// [`crate::generic::RigidBody`], a soft body's "shape" *is* its particle
/// positions, not a fixed collider transformed by one frame) connected by
/// [`Spring`]s. `inverse_masses[i] <= 0` pins particle `i` in place
/// (never moved by [`SoftBodyIntegrator::step`]) — the same "static via
/// zero/negative inverse mass" convention
/// [`crate::generic::RigidBody::inverse_mass`] uses.
#[derive(Debug, Clone)]
pub struct SoftBody<F: GaFlavor> {
    pub positions: Vec<F::Vector>,
    pub velocities: Vec<F::Vector>,
    pub inverse_masses: Vec<F::Scalar>,
    pub springs: Vec<Spring<F>>,
}

impl<F: GaFlavor> SoftBody<F> {
    pub fn particle_count(&self) -> usize {
        self.positions.len()
    }
}

fn axis_x<F: GaFlavor>() -> F::Vector {
    F::Vector::new(F::Scalar::ONE, F::Scalar::ZERO, F::Scalar::ZERO)
}

/// Advances every particle in a [`SoftBody`] by one timestep: spring
/// forces (structural + damping), gravity, semi-implicit Euler
/// integration, then a ground-plane collision pass. See the module doc
/// for why this doesn't reuse the rigid-body engine's collision types.
#[derive(Debug, Clone, Copy)]
pub struct SoftBodyIntegrator<F: GaFlavor> {
    pub gravity: F::Vector,
    pub ground: Plane<F>,
    /// `0` = fully inelastic (a particle stops dead at the ground), `1`
    /// = fully elastic (no speed lost bouncing off the ground) — same
    /// convention as [`crate::generic::ConstraintSolver::restitution`].
    pub restitution: F::Scalar,
}

impl<F: GaFlavor> SoftBodyIntegrator<F> {
    pub fn new(gravity: F::Vector, ground: Plane<F>, restitution: F::Scalar) -> Self {
        Self {
            gravity,
            ground,
            restitution,
        }
    }

    pub fn step(&self, body: &mut SoftBody<F>, dt: F::Scalar) {
        let mut forces = vec![F::Vector::ZERO; body.particle_count()];
        for spring in &body.springs {
            let pa = body.positions[spring.a];
            let pb = body.positions[spring.b];
            let delta = pb - pa;
            let dist = delta.length();
            let direction = if dist > F::Scalar::EPSILON {
                delta * (F::Scalar::ONE / dist)
            } else {
                axis_x::<F>()
            };
            let stretch = dist - spring.rest_length;
            let spring_force = direction * (spring.stiffness * stretch);

            let relative_velocity = body.velocities[spring.b] - body.velocities[spring.a];
            let closing_speed = relative_velocity.dot(direction);
            let damping_force = direction * (spring.damping * closing_speed);

            let total = spring_force + damping_force;
            forces[spring.a] = forces[spring.a] + total;
            forces[spring.b] = forces[spring.b] - total;
        }

        for i in 0..body.particle_count() {
            let inverse_mass = body.inverse_masses[i];
            if inverse_mass <= F::Scalar::ZERO {
                continue; // pinned particle
            }
            let acceleration = forces[i] * inverse_mass + self.gravity;
            body.velocities[i] = body.velocities[i] + acceleration * dt;
            body.positions[i] = body.positions[i] + body.velocities[i] * dt;

            let separation = self.ground.distance(body.positions[i]);
            if separation < F::Scalar::ZERO {
                body.positions[i] = body.positions[i] - self.ground.normal * separation;
                let normal_speed = body.velocities[i].dot(self.ground.normal);
                if normal_speed < F::Scalar::ZERO {
                    body.velocities[i] = body.velocities[i]
                        - self.ground.normal * (normal_speed * (F::Scalar::ONE + self.restitution));
                }
            }
        }
    }
}

/// Builds a spherical [`SoftBody`] from `gac-core`'s [`icosphere`] mesh
/// (`subdivisions`: `0` = the base 12-vertex icosahedron, each further
/// level roughly quadruples the triangle count), centered at `center`
/// with `radius`, plus one center particle spoked to every surface
/// vertex. `particle_mass`/`edge_stiffness`/`edge_damping` apply to every
/// surface particle/edge spring uniformly; `spoke_stiffness`/
/// `spoke_damping` (typically softer than the edges, so the ball can
/// compress before its surface visibly deforms) apply to the
/// center-to-surface spokes. The center particle's own mass is the sum
/// of the surface particles' masses (a resting-heavy "core" the surface
/// springs pull against) — pinning it instead (`inverse_mass = 0`) is a
/// reasonable alternative a caller can build directly via [`SoftBody`]'s
/// fields if a stiffer-feeling ball is wanted.
pub fn icosphere_soft_body<F: GaFlavor>(
    center: F::Vector,
    radius: F::Scalar,
    subdivisions: u32,
    particle_mass: F::Scalar,
    edge_stiffness: F::Scalar,
    edge_damping: F::Scalar,
    spoke_stiffness: F::Scalar,
    spoke_damping: F::Scalar,
) -> SoftBody<F> {
    let mesh = icosphere::<F>(subdivisions);

    let mut positions: Vec<F::Vector> = mesh.vertices.iter().map(|&d| center + d * radius).collect();
    let surface_count = positions.len();
    positions.push(center);
    let center_index = surface_count;

    let velocities = vec![F::Vector::ZERO; positions.len()];
    let inverse_masses: Vec<F::Scalar> = {
        let surface_inv = F::Scalar::ONE / particle_mass;
        let mut m = vec![surface_inv; surface_count];
        let center_mass = particle_mass * F::Scalar::from_f64(surface_count as f64);
        m.push(F::Scalar::ONE / center_mass);
        m
    };

    let mut springs = Vec::new();
    for (a, b) in mesh.unique_edges() {
        let rest_length = (positions[b] - positions[a]).length();
        springs.push(Spring {
            a,
            b,
            rest_length,
            stiffness: edge_stiffness,
            damping: edge_damping,
        });
    }
    for surface_index in 0..surface_count {
        let rest_length = (positions[surface_index] - positions[center_index]).length();
        springs.push(Spring {
            a: surface_index,
            b: center_index,
            rest_length,
            stiffness: spoke_stiffness,
            damping: spoke_damping,
        });
    }

    SoftBody {
        positions,
        velocities,
        inverse_masses,
        springs,
    }
}
