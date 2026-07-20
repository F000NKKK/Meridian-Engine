//! A mass-spring soft-body model — deformable bodies (a "rubber ball"
//! that squashes on impact and recovers), generic over `F: GaFlavor` for
//! the same reason [`crate::generic`]'s rigid-body engine is: nothing
//! here has a GPU-dispatch constraint of its own, so it's written once
//! rather than duplicated per scalar flavor (see CLAUDE.md's "Float/Fixed
//! branching" rule). [`super::float_softbody`]/[`super::fixed_softbody`]
//! alias these types the same way `crate::float`/`crate::fixed` alias the
//! rigid-body engine.
//!
//! [`icosphere_soft_body`] builds a ball: a subdivided icosahedron's
//! vertices as particles, its edges as structural springs (keep the
//! surface shape), plus one center particle connected to every surface
//! vertex by a spoke spring (keeps the ball from collapsing — without
//! spokes, a pure edge-spring surface has no resistance to being pushed
//! inward, since edge springs only resist *along the surface*, not
//! radially). [`SoftBodyIntegrator`] steps gravity, spring forces (Hooke's
//! law plus a velocity-relative damping term, since undamped springs
//! oscillate forever), and a simple ground-plane collision (push out of
//! penetration, reflect velocity with restitution) — deliberately not
//! reusing [`crate::generic`]'s rigid-body [`crate::generic::ConstraintSolver`]/
//! [`crate::generic::NarrowPhase`], which operate on whole rigid colliders
//! (spheres/cuboids), not individual particles; a soft body's collision
//! response is inherently per-particle.

use meridian_gac_core::generic::{GaFlavor, Plane, ScalarLike, VectorLike};

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

/// One triangular face of the icosphere mesh under construction, by
/// vertex index.
#[derive(Clone, Copy)]
struct Face {
    a: usize,
    b: usize,
    c: usize,
}

/// Builds a spherical [`SoftBody`]: a `subdivisions`-times-subdivided
/// icosahedron (`0` = the base 12-vertex icosahedron, each further level
/// roughly quadruples the triangle count) centered at `center` with
/// `radius`, plus one center particle spoked to every surface vertex.
/// `particle_mass`/`edge_stiffness`/`edge_damping` apply to every surface
/// particle/edge spring uniformly; `spoke_stiffness`/`spoke_damping`
/// (typically softer than the edges, so the ball can compress before its
/// surface visibly deforms) apply to the center-to-surface spokes. The
/// center particle's own mass is the sum of the surface particles' masses
/// (a resting-heavy "core" the surface springs pull against) — pinning it
/// instead (`inverse_mass = 0`) is a reasonable alternative a caller can
/// build directly via [`SoftBody`]'s fields if a stiffer-feeling ball is
/// wanted.
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
    let (directions, faces) = icosphere_mesh::<F>(subdivisions);

    let mut positions: Vec<F::Vector> = directions.iter().map(|&d| center + d * radius).collect();
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
    for (a, b) in unique_edges(&faces) {
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

/// The base icosahedron's 12 unit-length vertex directions and 20 faces
/// (the standard construction: all even permutations of `(0, ±1, ±φ)`
/// with the golden ratio `φ`, normalized), then `subdivisions` rounds of
/// "split every triangle into 4 by its edge midpoints, re-normalized to
/// the unit sphere."
fn icosphere_mesh<F: GaFlavor>(subdivisions: u32) -> (Vec<F::Vector>, Vec<Face>) {
    let phi = (F::Scalar::ONE + F::Scalar::from_f64(5.0).sqrt()) / (F::Scalar::ONE + F::Scalar::ONE);
    let raw: [[F::Scalar; 3]; 12] = {
        let z = F::Scalar::ZERO;
        let o = F::Scalar::ONE;
        [
            [-o, phi, z],
            [o, phi, z],
            [-o, -phi, z],
            [o, -phi, z],
            [z, -o, phi],
            [z, o, phi],
            [z, -o, -phi],
            [z, o, -phi],
            [phi, z, -o],
            [phi, z, o],
            [-phi, z, -o],
            [-phi, z, o],
        ]
    };
    let mut vertices: Vec<F::Vector> = raw
        .iter()
        .map(|v| F::Vector::new(v[0], v[1], v[2]).normalize())
        .collect();

    let mut faces = vec![
        Face { a: 0, b: 11, c: 5 },
        Face { a: 0, b: 5, c: 1 },
        Face { a: 0, b: 1, c: 7 },
        Face { a: 0, b: 7, c: 10 },
        Face { a: 0, b: 10, c: 11 },
        Face { a: 1, b: 5, c: 9 },
        Face { a: 5, b: 11, c: 4 },
        Face { a: 11, b: 10, c: 2 },
        Face { a: 10, b: 7, c: 6 },
        Face { a: 7, b: 1, c: 8 },
        Face { a: 3, b: 9, c: 4 },
        Face { a: 3, b: 4, c: 2 },
        Face { a: 3, b: 2, c: 6 },
        Face { a: 3, b: 6, c: 8 },
        Face { a: 3, b: 8, c: 9 },
        Face { a: 4, b: 9, c: 5 },
        Face { a: 2, b: 4, c: 11 },
        Face { a: 6, b: 2, c: 10 },
        Face { a: 8, b: 6, c: 7 },
        Face { a: 9, b: 8, c: 1 },
    ];

    for _ in 0..subdivisions {
        let mut midpoint_cache = std::collections::HashMap::new();
        let mut next_faces = Vec::with_capacity(faces.len() * 4);
        for face in &faces {
            let ab = midpoint::<F>(&mut vertices, &mut midpoint_cache, face.a, face.b);
            let bc = midpoint::<F>(&mut vertices, &mut midpoint_cache, face.b, face.c);
            let ca = midpoint::<F>(&mut vertices, &mut midpoint_cache, face.c, face.a);
            next_faces.push(Face {
                a: face.a,
                b: ab,
                c: ca,
            });
            next_faces.push(Face {
                a: face.b,
                b: bc,
                c: ab,
            });
            next_faces.push(Face {
                a: face.c,
                b: ca,
                c: bc,
            });
            next_faces.push(Face { a: ab, b: bc, c: ca });
        }
        faces = next_faces;
    }

    (vertices, faces)
}

/// Returns the index of the unit-sphere midpoint between vertices `i`/`j`
/// — reused from `cache` if this edge was already split (shared between
/// two adjacent triangles), otherwise computed and cached.
fn midpoint<F: GaFlavor>(
    vertices: &mut Vec<F::Vector>,
    cache: &mut std::collections::HashMap<(usize, usize), usize>,
    i: usize,
    j: usize,
) -> usize {
    let key = if i < j { (i, j) } else { (j, i) };
    if let Some(&existing) = cache.get(&key) {
        return existing;
    }
    let mid = (vertices[i] + vertices[j]).normalize();
    let index = vertices.len();
    vertices.push(mid);
    cache.insert(key, index);
    index
}

/// Every edge of `faces`, each exactly once regardless of how many
/// triangles share it (every icosphere edge is shared by exactly 2).
fn unique_edges(faces: &[Face]) -> Vec<(usize, usize)> {
    let mut seen = std::collections::HashSet::new();
    let mut edges = Vec::new();
    let mut add = |a: usize,
                   b: usize,
                   seen: &mut std::collections::HashSet<(usize, usize)>,
                   edges: &mut Vec<(usize, usize)>| {
        let key = if a < b { (a, b) } else { (b, a) };
        if seen.insert(key) {
            edges.push(key);
        }
    };
    for face in faces {
        add(face.a, face.b, &mut seen, &mut edges);
        add(face.b, face.c, &mut seen, &mut edges);
        add(face.c, face.a, &mut seen, &mut edges);
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_gac_core::float_ga::FloatFlavor;

    #[test]
    fn icosphere_mesh_has_no_duplicate_or_self_edges() {
        let (vertices, faces) = icosphere_mesh::<FloatFlavor>(1);
        let edges = unique_edges(&faces);
        for &(a, b) in &edges {
            assert_ne!(a, b, "an edge must connect two distinct vertices");
            assert!(a < vertices.len() && b < vertices.len());
        }
        let mut sorted = edges.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            edges.len(),
            "unique_edges must not report the same edge twice"
        );
    }

    #[test]
    fn icosphere_mesh_vertices_lie_on_the_unit_sphere() {
        let (vertices, _) = icosphere_mesh::<FloatFlavor>(1);
        for v in vertices {
            assert!(
                (v.length() - 1.0).abs() < 1e-4,
                "every icosphere vertex must be unit length, got {v:?} (length {})",
                v.length()
            );
        }
    }
}
