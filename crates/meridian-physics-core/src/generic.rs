//! Generic rigid-body physics engine — [`RigidBody`], [`ColliderShape`],
//! [`Contact`], [`BroadPhase`], [`NarrowPhase`], [`ConstraintSolver`] and
//! [`Integrator`], all written once, generic over `F: GaFlavor`.
//!
//! Unlike `gac-core::Motor3`/`Vec3`/`Bivector3`/`Rotor` (concretely
//! duplicated between `float_ga` and `fixed_ga` because
//! `meridian-gac-compute` dispatches them to a GPU that has no real
//! `i64`), nothing in this engine has a GPU-dispatch constraint of its
//! own — broad phase, narrow phase, constraint solving and integration
//! are the same sequence of operations regardless of which scalar type
//! backs them. So unlike `gac-core`'s `float_ga`/`fixed_ga` split, this
//! crate's `float.rs`/`fixed.rs` are thin `FloatFlavor`/`FixedFlavor`
//! aliases over the one engine defined here — see CLAUDE.md's
//! "Float/Fixed branching" rule.
//!
//! Deliberately its own module, not defined directly at the crate root
//! next to `pub use float::*;` — a locally-defined item always shadows a
//! glob-imported same-named item in Rust, so `RigidBody<F>` living at
//! the crate root would silently shadow `float::RigidBody`'s re-export
//! and break every existing unparameterized `meridian_physics_core::RigidBody`
//! call site. See `gac-core::generic`'s doc comment for the same bug,
//! caught and fixed there first.

use meridian_gac_core::generic::{
    Aabb, BivectorLike, GaFlavor, MotorLike, Obb, ScalarLike, Shape, VectorLike,
};
use meridian_resource_core::ResourceId;

/// Marker type for collider-mesh `ResourceId`s — see
/// docs/adr/006-resource-core-separation.md.
pub struct ColliderMeshMarker;
pub type ColliderMeshHandle = ResourceId<ColliderMeshMarker>;

fn axis_x<F: GaFlavor>() -> F::Vector {
    F::Vector::new(F::Scalar::ONE, F::Scalar::ZERO, F::Scalar::ZERO)
}
fn axis_y<F: GaFlavor>() -> F::Vector {
    F::Vector::new(F::Scalar::ZERO, F::Scalar::ONE, F::Scalar::ZERO)
}
fn axis_z<F: GaFlavor>() -> F::Vector {
    F::Vector::new(F::Scalar::ZERO, F::Scalar::ZERO, F::Scalar::ONE)
}

/// A collision shape. `Sphere` and `Cuboid` (`gac-core::Obb`'s
/// half-extents, oriented by the owning `RigidBody`'s own `frame` — no
/// separate orientation to keep in sync) so far; capsule/mesh (via
/// [`ColliderMeshHandle`]) are additive later.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColliderShape<F: GaFlavor> {
    Sphere { radius: F::Scalar },
    Cuboid { half_extents: F::Vector },
}

impl<F: GaFlavor> Default for ColliderShape<F> {
    fn default() -> Self {
        ColliderShape::Sphere {
            radius: F::Scalar::from_f64(0.5),
        }
    }
}

/// A simulated rigid body: spatial frame (shared with every other
/// subsystem via the GAC) + linear and angular state. `mass <= 0` means
/// static/immovable (infinite mass) — never touched by [`Integrator`] or
/// given any velocity change by [`ConstraintSolver`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigidBody<F: GaFlavor> {
    pub frame: F::Motor,
    pub velocity: F::Vector,
    pub angular_velocity: F::Bivector,
    pub mass: F::Scalar,
    pub shape: ColliderShape<F>,
}

impl<F: GaFlavor> Default for RigidBody<F> {
    fn default() -> Self {
        Self {
            frame: F::Motor::identity(),
            velocity: F::Vector::ZERO,
            angular_velocity: F::Bivector::ZERO,
            mass: F::Scalar::ZERO,
            shape: ColliderShape::default(),
        }
    }
}

impl<F: GaFlavor> RigidBody<F> {
    pub fn inverse_mass(&self) -> F::Scalar {
        if self.mass > F::Scalar::ZERO {
            F::Scalar::ONE / self.mass
        } else {
            F::Scalar::ZERO
        }
    }

    /// Moment of inertia about the body's center of mass. `Sphere` is
    /// exact (`(2/5) * m * r²`, isotropic by construction). `Cuboid` is
    /// *not* exact — see `docs/physics-design.md`; this returns the
    /// average of the three principal moments, `(2/9) * m * (hx² + hy² +
    /// hz²)`, since [`ConstraintSolver`]'s angular response only has a
    /// single scalar `inverse_inertia` to work with.
    pub fn moment_of_inertia(&self) -> F::Scalar {
        match self.shape {
            ColliderShape::Sphere { radius } => {
                F::Scalar::from_f64(0.4) * self.mass * radius * radius
            }
            ColliderShape::Cuboid { half_extents } => {
                F::Scalar::from_f64(2.0 / 9.0)
                    * self.mass
                    * (half_extents.x() * half_extents.x()
                        + half_extents.y() * half_extents.y()
                        + half_extents.z() * half_extents.z())
            }
        }
    }

    /// This body's collider as a world-space [`Obb`] — meaningful for
    /// `Cuboid` bodies (a `Sphere` has no orientation to speak of, so
    /// there's no equivalent method for it; [`RigidBody::position`] plus
    /// the collider's radius is all a sphere needs).
    pub fn as_obb(&self, half_extents: F::Vector) -> Obb<F> {
        Obb {
            frame: self.frame,
            half_extents,
        }
    }

    pub fn inverse_inertia(&self) -> F::Scalar {
        let i = self.moment_of_inertia();
        if i > F::Scalar::ZERO {
            F::Scalar::ONE / i
        } else {
            F::Scalar::ZERO
        }
    }

    pub fn position(&self) -> F::Vector {
        self.frame.transform_point(F::Vector::ZERO)
    }
}

fn aabb_of<F: GaFlavor>(body: &RigidBody<F>) -> Aabb<F> {
    match body.shape {
        ColliderShape::Sphere { radius } => Aabb::from_sphere(body.position(), radius),
        ColliderShape::Cuboid { half_extents } => {
            // The world-space AABB of an oriented box: query the box's own
            // support function along each world axis — exactly the same
            // `Shape` interface `ConvexVolume`/`Frustum` use, applied here
            // to a different generic problem (bounding an oriented shape),
            // not a bespoke box-AABB formula.
            let obb = body.as_obb(half_extents);
            let (x, y, z) = (axis_x::<F>(), axis_y::<F>(), axis_z::<F>());
            Aabb {
                min: F::Vector::new(
                    obb.support(-x).x(),
                    obb.support(-y).y(),
                    obb.support(-z).z(),
                ),
                max: F::Vector::new(obb.support(x).x(), obb.support(y).y(), obb.support(z).z()),
            }
        }
    }
}

/// A broad-phase acceleration structure (BVH, spatial hash, ...). Owned
/// here, not in `physics-driver` — see docs/physics-design.md. Currently a
/// naive O(n²) AABB sweep.
#[derive(Debug)]
pub struct BroadPhase<F: GaFlavor> {
    pairs: Vec<(usize, usize)>,
    _flavor: core::marker::PhantomData<F>,
}

impl<F: GaFlavor> Default for BroadPhase<F> {
    fn default() -> Self {
        Self {
            pairs: Vec::new(),
            _flavor: core::marker::PhantomData,
        }
    }
}

impl<F: GaFlavor> BroadPhase<F> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns index pairs into `bodies` whose AABBs overlap — candidates
    /// for narrow-phase, not confirmed contacts.
    pub fn find_candidate_pairs(&mut self, bodies: &[RigidBody<F>]) -> &[(usize, usize)] {
        self.pairs.clear();
        let aabbs: Vec<Aabb<F>> = bodies.iter().map(aabb_of).collect();
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact<F: GaFlavor> {
    pub a: usize,
    pub b: usize,
    pub normal: F::Vector,
    pub penetration: F::Scalar,
    pub point: F::Vector,
}

impl<F: GaFlavor> Default for Contact<F> {
    fn default() -> Self {
        Self {
            a: 0,
            b: 0,
            normal: F::Vector::ZERO,
            penetration: F::Scalar::ZERO,
            point: F::Vector::ZERO,
        }
    }
}

/// Sphere-sphere exact test. `None` if not overlapping.
fn sphere_vs_sphere<F: GaFlavor>(
    pa: F::Vector,
    ra: F::Scalar,
    pb: F::Vector,
    rb: F::Scalar,
) -> Option<(F::Vector, F::Scalar, F::Vector)> {
    let delta = pb - pa;
    let dist = delta.length();
    let combined = ra + rb;

    if dist >= combined {
        return None;
    }
    // Centers coincide exactly: pick an arbitrary separating axis rather
    // than dividing by a zero-length delta.
    let normal = if dist > F::Scalar::EPSILON {
        delta * (F::Scalar::ONE / dist)
    } else {
        axis_x::<F>()
    };
    let point = pa + normal * ra;
    Some((point, combined - dist, normal))
}

/// Closest point on `obb`'s surface to `point`, the outward normal from
/// the box to that point, and the separation along that normal (negative
/// means `point` is embedded inside the box).
///
/// The embedded case needs its own branch: clamping `point`'s local
/// coordinates to the box's extents is exactly the closest-surface-point
/// formula when `point` is *outside*, but when it's already inside,
/// clamping is a no-op (the point doesn't move) and the "normal" from a
/// zero-length delta would be undefined — so this pushes out through
/// whichever face is nearest instead.
fn closest_point_on_obb<F: GaFlavor>(
    obb: &Obb<F>,
    point: F::Vector,
) -> (F::Vector, F::Vector, F::Scalar) {
    let local = obb.frame.inverse().transform_point(point);
    let inside = local.x().abs() <= obb.half_extents.x()
        && local.y().abs() <= obb.half_extents.y()
        && local.z().abs() <= obb.half_extents.z();

    if inside {
        let dx = obb.half_extents.x() - local.x().abs();
        let dy = obb.half_extents.y() - local.y().abs();
        let dz = obb.half_extents.z() - local.z().abs();
        let (local_normal, depth) = if dx <= dy && dx <= dz {
            (
                F::Vector::new(local.x().signum(), F::Scalar::ZERO, F::Scalar::ZERO),
                dx,
            )
        } else if dy <= dz {
            (
                F::Vector::new(F::Scalar::ZERO, local.y().signum(), F::Scalar::ZERO),
                dy,
            )
        } else {
            (
                F::Vector::new(F::Scalar::ZERO, F::Scalar::ZERO, local.z().signum()),
                dz,
            )
        };
        let closest_local = F::Vector::new(
            if local_normal.x() != F::Scalar::ZERO {
                obb.half_extents.x() * local_normal.x()
            } else {
                local.x()
            },
            if local_normal.y() != F::Scalar::ZERO {
                obb.half_extents.y() * local_normal.y()
            } else {
                local.y()
            },
            if local_normal.z() != F::Scalar::ZERO {
                obb.half_extents.z() * local_normal.z()
            } else {
                local.z()
            },
        );
        let closest_world = obb.frame.transform_point(closest_local);
        (
            closest_world,
            obb.frame.transform_vector(local_normal),
            -depth,
        )
    } else {
        let clamped = F::Vector::new(
            local.x().clamp(-obb.half_extents.x(), obb.half_extents.x()),
            local.y().clamp(-obb.half_extents.y(), obb.half_extents.y()),
            local.z().clamp(-obb.half_extents.z(), obb.half_extents.z()),
        );
        let closest_world = obb.frame.transform_point(clamped);
        let delta = point - closest_world;
        let dist = delta.length();
        let normal = if dist > F::Scalar::EPSILON {
            delta * (F::Scalar::ONE / dist)
        } else {
            axis_x::<F>()
        };
        (closest_world, normal, dist)
    }
}

/// Sphere-cuboid exact test via [`closest_point_on_obb`]. Returns the
/// contact point, outward normal (cuboid toward sphere), and penetration.
fn sphere_vs_cuboid<F: GaFlavor>(
    sphere_center: F::Vector,
    radius: F::Scalar,
    obb: &Obb<F>,
) -> Option<(F::Vector, F::Scalar, F::Vector)> {
    let (closest, normal, separation) = closest_point_on_obb(obb, sphere_center);
    let penetration = radius - separation;
    if penetration <= F::Scalar::ZERO {
        return None;
    }
    Some((closest, penetration, normal))
}

/// The box's own local axes (its rotated X/Y/Z), unit length since
/// `transform_vector` only rotates.
fn obb_axes<F: GaFlavor>(obb: &Obb<F>) -> [F::Vector; 3] {
    [
        obb.frame.transform_vector(axis_x::<F>()),
        obb.frame.transform_vector(axis_y::<F>()),
        obb.frame.transform_vector(axis_z::<F>()),
    ]
}

/// How far `obb` extends to either side of its center when projected onto
/// `axis` (`axis` must be unit length).
fn projected_half_width<F: GaFlavor>(
    axis: F::Vector,
    axes: &[F::Vector; 3],
    half_extents: F::Vector,
) -> F::Scalar {
    axes[0].dot(axis).abs() * half_extents.x()
        + axes[1].dot(axis).abs() * half_extents.y()
        + axes[2].dot(axis).abs() * half_extents.z()
}

/// Which body's face axis (if any) the SAT search's least-overlap axis
/// came from — determines whether [`cuboid_vs_cuboid`] can build a real
/// face-face manifold (see [`face_manifold`]) or has to fall back to a
/// single edge/corner contact point.
enum SatAxisOwner {
    A(usize),
    B(usize),
    Edge,
}

/// All 8 world-space corners of `obb`.
fn obb_corners<F: GaFlavor>(obb: &Obb<F>) -> [F::Vector; 8] {
    let (hx, hy, hz) = (
        obb.half_extents.x(),
        obb.half_extents.y(),
        obb.half_extents.z(),
    );
    let mut corners = [F::Vector::ZERO; 8];
    let mut i = 0;
    for sx in [-hx, hx] {
        for sy in [-hy, hy] {
            for sz in [-hz, hz] {
                corners[i] = obb.frame.transform_point(F::Vector::new(sx, sy, sz));
                i += 1;
            }
        }
    }
    corners
}

/// A same-face-index accessor for `half_extents`/`local` components —
/// `axis_index` is 0/1/2 for the reference box's local x/y/z, matching
/// [`obb_axes`]'s ordering.
fn component_at<F: GaFlavor>(v: F::Vector, axis_index: usize) -> F::Scalar {
    match axis_index {
        0 => v.x(),
        1 => v.y(),
        _ => v.z(),
    }
}

/// Builds a (typically 4-point, sometimes fewer) contact manifold for a
/// face-face box-box contact: `reference`'s local axis `axis_index`
/// matches the SAT normal, so `reference`'s face along that axis is the
/// reference face. `incident`'s corners nearest that face (found by
/// sorting on the same axis in `reference`'s local frame) are clipped
/// against the reference face's lateral bounds — a simplified manifold
/// (full Sutherland-Hodgman polygon clipping would handle partial
/// overlaps more precisely, but for the common resting-box case, "which
/// of the incident box's corners land inside the reference face's
/// footprint" is exact). `None` if no incident corner survives the
/// lateral-bounds test (an edge/corner-only contact, not a face-face
/// one) — the caller falls back to the old single-point formula.
///
/// Without this, `NarrowPhase` reported exactly one contact point per
/// box-box pair (the midpoint of each box's own support point along the
/// SAT normal — see the removed single-point formula this replaced).
/// For a box resting flat on a floor, that single point is one of the
/// box's corners, whose identity flips between frames as the box rocks
/// even slightly — `ConstraintSolver::resolve`'s torque term
/// (`wedge(contact.point - position, impulse)`) then injects spurious,
/// flip-flopping angular impulse every frame, which is exactly the
/// "cube/pyramid jitter and launch themselves" bug this fixes. Four
/// stable corner contacts (matching the real overlap footprint) instead
/// keep that offset's horizontal component consistent frame to frame,
/// so the torque it produces settles instead of oscillating.
fn face_manifold<F: GaFlavor>(
    reference: &Obb<F>,
    axis_index: usize,
    incident: &Obb<F>,
) -> Option<Vec<F::Vector>> {
    let he_ref = reference.half_extents;
    let lateral: [usize; 2] = match axis_index {
        0 => [1, 2],
        1 => [0, 2],
        _ => [0, 1],
    };
    let tolerance = F::Scalar::from_f64(0.05);

    let inv_reference = reference.frame.inverse();
    let incident_center_local =
        inv_reference.transform_point(incident.frame.transform_point(F::Vector::ZERO));
    let incident_on_positive_side =
        component_at::<F>(incident_center_local, axis_index) >= F::Scalar::ZERO;

    let mut by_depth: Vec<(F::Vector, F::Scalar)> = obb_corners(incident)
        .into_iter()
        .map(|world| {
            let local = inv_reference.transform_point(world);
            (world, component_at::<F>(local, axis_index))
        })
        .collect();
    // The 4 corners of the incident face nearest the reference face are
    // the ones with the smallest (if incident sits on the +axis side) or
    // largest (if on the -axis side) depth coordinate in reference-local
    // space.
    by_depth.sort_by(|(_, da), (_, db)| {
        if incident_on_positive_side {
            da.partial_cmp(db).unwrap_or(core::cmp::Ordering::Equal)
        } else {
            db.partial_cmp(da).unwrap_or(core::cmp::Ordering::Equal)
        }
    });

    let points: Vec<F::Vector> = by_depth
        .into_iter()
        .take(4)
        .filter(|&(world, _)| {
            let local = inv_reference.transform_point(world);
            component_at::<F>(local, lateral[0]).abs()
                <= component_at::<F>(he_ref, lateral[0]) + tolerance
                && component_at::<F>(local, lateral[1]).abs()
                    <= component_at::<F>(he_ref, lateral[1]) + tolerance
        })
        .map(|(world, _)| world)
        .collect();

    if points.is_empty() {
        None
    } else {
        Some(points)
    }
}

/// Cuboid-cuboid exact test via the separating axis theorem (SAT): a
/// convex polyhedron pair is disjoint iff some axis exists — always one of
/// the 6 face normals or 9 face-normal cross products for a box pair —
/// along which their projections don't overlap. If every candidate axis
/// overlaps, the pair intersects, and the axis with the *least* overlap is
/// the standard contact normal (the direction resolving the penetration
/// with the smallest push).
///
/// The contact manifold is built by [`face_manifold`] when the winning
/// axis is one box's own face axis (the common case: a box resting on,
/// or pressed flat against, another box or the floor) — see that
/// function's doc comment for why a single point isn't enough. When the
/// winning axis instead comes from an edge-edge cross product (a true
/// edge/corner contact), a single point — the midpoint between each
/// box's own support point along the normal, via the same
/// [`Shape::support`] interface `ConvexVolume`/`Frustum` are built on —
/// is the geometrically correct manifold; there's no face to clip
/// against. Returns every manifold point sharing this pair's one
/// `normal` and total `penetration` (not divided per point — see
/// [`NarrowPhase::generate_contacts`], the caller that divides it when
/// expanding one pair into multiple [`Contact`]s, so the sum of
/// per-point positional corrections reproduces the same total magnitude
/// a single-point contact would have applied instead of over-correcting
/// once per extra point).
type Manifold<F> = (
    Vec<<F as GaFlavor>::Vector>,
    <F as GaFlavor>::Scalar,
    <F as GaFlavor>::Vector,
);

fn cuboid_vs_cuboid<F: GaFlavor>(obb_a: &Obb<F>, obb_b: &Obb<F>) -> Option<Manifold<F>> {
    let axes_a = obb_axes(obb_a);
    let axes_b = obb_axes(obb_b);
    let center_delta =
        obb_b.frame.transform_point(F::Vector::ZERO) - obb_a.frame.transform_point(F::Vector::ZERO);

    let mut candidate_axes: Vec<(F::Vector, SatAxisOwner)> = Vec::with_capacity(15);
    for (i, &axis) in axes_a.iter().enumerate() {
        candidate_axes.push((axis, SatAxisOwner::A(i)));
    }
    for (i, &axis) in axes_b.iter().enumerate() {
        candidate_axes.push((axis, SatAxisOwner::B(i)));
    }
    for &ai in &axes_a {
        for &bi in &axes_b {
            let cross = ai.cross(bi);
            // Near-parallel edges: the cross product is ~zero and this
            // axis is redundant with the face-normal axes already tested.
            if cross.length() > F::Scalar::EPSILON {
                candidate_axes.push((cross.normalize(), SatAxisOwner::Edge));
            }
        }
    }

    let mut min_overlap = F::Scalar::MAX;
    let mut best_axis = axis_x::<F>();
    let mut best_owner = SatAxisOwner::Edge;
    for (axis, owner) in candidate_axes {
        let half_width_a = projected_half_width::<F>(axis, &axes_a, obb_a.half_extents);
        let half_width_b = projected_half_width::<F>(axis, &axes_b, obb_b.half_extents);
        let separation = center_delta.dot(axis).abs();
        let overlap = half_width_a + half_width_b - separation;
        if overlap < F::Scalar::ZERO {
            return None; // A separating axis exists: the boxes don't touch.
        }
        if overlap < min_overlap {
            min_overlap = overlap;
            best_axis = axis;
            best_owner = owner;
        }
    }

    // Orient the normal from A toward B.
    let normal = if center_delta.dot(best_axis) >= F::Scalar::ZERO {
        best_axis
    } else {
        -best_axis
    };

    let manifold = match best_owner {
        SatAxisOwner::A(axis_index) => face_manifold::<F>(obb_a, axis_index, obb_b),
        SatAxisOwner::B(axis_index) => face_manifold::<F>(obb_b, axis_index, obb_a),
        SatAxisOwner::Edge => None,
    };
    let points = manifold.unwrap_or_else(|| {
        let two = F::Scalar::ONE + F::Scalar::ONE;
        vec![(obb_a.support(normal) + obb_b.support(-normal)) * (F::Scalar::ONE / two)]
    });

    Some((points, min_overlap, normal))
}

/// Resolves candidate pairs from [`BroadPhase`] into exact [`Contact`]s.
#[derive(Debug)]
pub struct NarrowPhase<F: GaFlavor>(core::marker::PhantomData<F>);

impl<F: GaFlavor> Default for NarrowPhase<F> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<F: GaFlavor> NarrowPhase<F> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Exact test for the current pair of [`ColliderShape`]s. `None` if
    /// not overlapping.
    pub fn test_pair(&self, bodies: &[RigidBody<F>], a: usize, b: usize) -> Option<Contact<F>> {
        let (point, penetration, normal) = match (bodies[a].shape, bodies[b].shape) {
            (ColliderShape::Sphere { radius: ra }, ColliderShape::Sphere { radius: rb }) => {
                sphere_vs_sphere::<F>(bodies[a].position(), ra, bodies[b].position(), rb)?
            }
            (ColliderShape::Sphere { radius }, ColliderShape::Cuboid { half_extents }) => {
                let obb = bodies[b].as_obb(half_extents);
                let (point, penetration, normal) =
                    sphere_vs_cuboid(bodies[a].position(), radius, &obb)?;
                // sphere_vs_cuboid's normal points cuboid(b) -> sphere(a);
                // Contact.normal must point a -> b, so flip it.
                (point, penetration, -normal)
            }
            (ColliderShape::Cuboid { half_extents }, ColliderShape::Sphere { radius }) => {
                let obb = bodies[a].as_obb(half_extents);
                // Already cuboid(a) -> sphere(b) = a -> b: no flip needed.
                sphere_vs_cuboid(bodies[b].position(), radius, &obb)?
            }
            (
                ColliderShape::Cuboid {
                    half_extents: half_extents_a,
                },
                ColliderShape::Cuboid {
                    half_extents: half_extents_b,
                },
            ) => {
                let obb_a = bodies[a].as_obb(half_extents_a);
                let obb_b = bodies[b].as_obb(half_extents_b);
                // `test_pair` reports the pair's total, undivided
                // penetration and one representative manifold point —
                // see `generate_contacts` for the multi-point expansion
                // this single-`Contact` view intentionally doesn't do.
                let (points, penetration, normal) = cuboid_vs_cuboid(&obb_a, &obb_b)?;
                (points[0], penetration, normal)
            }
        };
        Some(Contact {
            a,
            b,
            normal,
            penetration,
            point,
        })
    }

    /// Like [`test_pair`](Self::test_pair), but expands a box-box pair's
    /// full contact manifold (see [`face_manifold`]) into one [`Contact`]
    /// per manifold point instead of collapsing it to one — this is what
    /// [`ConstraintSolver::resolve`] should iterate over; `test_pair`'s
    /// single-point view exists only for simple exact-overlap queries.
    pub fn generate_contacts(
        &self,
        bodies: &[RigidBody<F>],
        candidate_pairs: &[(usize, usize)],
    ) -> Vec<Contact<F>> {
        let mut contacts = Vec::with_capacity(candidate_pairs.len());
        for &(a, b) in candidate_pairs {
            match (bodies[a].shape, bodies[b].shape) {
                (
                    ColliderShape::Cuboid { half_extents: he_a },
                    ColliderShape::Cuboid { half_extents: he_b },
                ) => {
                    let obb_a = bodies[a].as_obb(he_a);
                    let obb_b = bodies[b].as_obb(he_b);
                    let Some((points, penetration, normal)) = cuboid_vs_cuboid(&obb_a, &obb_b)
                    else {
                        continue;
                    };
                    let point_count = F::Scalar::from_f64(points.len() as f64);
                    for point in points {
                        contacts.push(Contact {
                            a,
                            b,
                            normal,
                            penetration: penetration / point_count,
                            point,
                        });
                    }
                }
                _ => {
                    if let Some(contact) = self.test_pair(bodies, a, b) {
                        contacts.push(contact);
                    }
                }
            }
        }
        contacts
    }
}

/// Resolves contacts into corrective linear *and* angular impulses
/// (conserving momentum) plus a small positional correction so resting
/// contacts don't sink into each other frame over frame.
///
/// The angular part is a simplified, not fully coupled, solver — see
/// docs/physics-design.md for why (it's provably inert for sphere-sphere
/// contacts specifically, since the contact point always lies on the
/// line between the two centers).
#[derive(Debug, Clone, Copy)]
pub struct ConstraintSolver<F: GaFlavor> {
    /// 0 = perfectly inelastic (bodies stick, no bounce), 1 = perfectly
    /// elastic (no kinetic energy lost).
    pub restitution: F::Scalar,
    /// Coulomb friction coefficient, applied as a tangential impulse
    /// clamped to `friction * normal_impulse` (see [`resolve`](Self::resolve)) —
    /// `0` disables friction entirely (the old behavior every existing
    /// [`ConstraintSolver::new`] call site got, since this field didn't
    /// exist before). Not a true static/dynamic-friction split (that
    /// needs per-contact tangential *position* tracking to detect
    /// "still stuck" vs "sliding", which this solver's single-point,
    /// no-warm-starting design doesn't carry) — one coefficient used as
    /// a dynamic-friction cone for both regimes, the same simplification
    /// most simple/game physics engines make.
    pub friction: F::Scalar,
}

impl<F: GaFlavor> Default for ConstraintSolver<F> {
    fn default() -> Self {
        Self {
            restitution: F::Scalar::ZERO,
            friction: F::Scalar::ZERO,
        }
    }
}

impl<F: GaFlavor> ConstraintSolver<F> {
    /// `friction` defaults to `0` (no friction) — existing call sites
    /// that only ever passed `restitution` keep their old behavior
    /// unchanged; opt into friction via
    /// [`with_friction`](Self::with_friction).
    pub fn new(restitution: F::Scalar) -> Self {
        Self {
            restitution,
            friction: F::Scalar::ZERO,
        }
    }

    pub fn with_friction(mut self, friction: F::Scalar) -> Self {
        self.friction = friction;
        self
    }

    /// Applies an impulse-based resolution for `contact`, mutating only
    /// `bodies[contact.a]` and `bodies[contact.b]`. A no-op if both bodies
    /// are static (infinite mass on both sides — nothing to resolve).
    pub fn resolve(&self, bodies: &mut [RigidBody<F>], contact: &Contact<F>) {
        let inv_mass_a = bodies[contact.a].inverse_mass();
        let inv_mass_b = bodies[contact.b].inverse_mass();
        let total_inv_mass = inv_mass_a + inv_mass_b;
        if total_inv_mass <= F::Scalar::ZERO {
            return;
        }

        let relative_velocity = bodies[contact.b].velocity - bodies[contact.a].velocity;
        let velocity_along_normal = relative_velocity.dot(contact.normal);

        if velocity_along_normal < F::Scalar::ZERO {
            let j = -(F::Scalar::ONE + self.restitution) * velocity_along_normal / total_inv_mass;
            let impulse = contact.normal * j;
            bodies[contact.a].velocity = bodies[contact.a].velocity - impulse * inv_mass_a;
            bodies[contact.b].velocity = bodies[contact.b].velocity + impulse * inv_mass_b;

            // Angular response — see the type's doc comment on why this
            // is a simplified, decoupled solve.
            let offset_a = contact.point - bodies[contact.a].position();
            let offset_b = contact.point - bodies[contact.b].position();
            let torque_a = F::Bivector::wedge(offset_a, impulse);
            let torque_b = F::Bivector::wedge(offset_b, impulse);
            let inv_inertia_a = bodies[contact.a].inverse_inertia();
            let inv_inertia_b = bodies[contact.b].inverse_inertia();
            bodies[contact.a].angular_velocity =
                bodies[contact.a].angular_velocity - torque_a * inv_inertia_a;
            bodies[contact.b].angular_velocity =
                bodies[contact.b].angular_velocity + torque_b * inv_inertia_b;

            // Coulomb friction: the tangential component of relative
            // velocity (whatever's left of `relative_velocity` once the
            // along-normal part is removed) gets its own impulse,
            // opposing sliding, clamped to `friction * j` — a cone
            // around the normal impulse, not a fixed force, so light
            // contacts can't get an unphysically strong sideways
            // impulse. Recomputed from the *post-normal-impulse*
            // velocity so it reacts to what the normal impulse just
            // did, same sequential-impulse spirit as running each
            // manifold point's `resolve` call in sequence.
            if self.friction > F::Scalar::ZERO {
                let relative_velocity = bodies[contact.b].velocity - bodies[contact.a].velocity;
                let tangential_velocity =
                    relative_velocity - contact.normal * relative_velocity.dot(contact.normal);
                let tangential_speed = tangential_velocity.length();
                if tangential_speed > F::Scalar::EPSILON {
                    let tangent = tangential_velocity * (F::Scalar::ONE / tangential_speed);
                    let jt = (-tangential_speed / total_inv_mass)
                        .clamp(-self.friction * j, self.friction * j);
                    let friction_impulse = tangent * jt;
                    bodies[contact.a].velocity =
                        bodies[contact.a].velocity - friction_impulse * inv_mass_a;
                    bodies[contact.b].velocity =
                        bodies[contact.b].velocity + friction_impulse * inv_mass_b;

                    let friction_torque_a = F::Bivector::wedge(offset_a, friction_impulse);
                    let friction_torque_b = F::Bivector::wedge(offset_b, friction_impulse);
                    bodies[contact.a].angular_velocity =
                        bodies[contact.a].angular_velocity - friction_torque_a * inv_inertia_a;
                    bodies[contact.b].angular_velocity =
                        bodies[contact.b].angular_velocity + friction_torque_b * inv_inertia_b;
                }
            }
        }

        let correction_percent = F::Scalar::from_f64(0.8);
        let slop = F::Scalar::from_f64(0.01);
        let correction_mag =
            (contact.penetration - slop).max(F::Scalar::ZERO) / total_inv_mass * correction_percent;
        let correction = contact.normal * correction_mag;
        bodies[contact.a].frame = bodies[contact.a]
            .frame
            .compose(F::Motor::translation(correction * -inv_mass_a));
        bodies[contact.b].frame = bodies[contact.b]
            .frame
            .compose(F::Motor::translation(correction * inv_mass_b));
    }
}

/// Advances rigid body velocity/angular velocity and position/orientation
/// by one timestep (semi-implicit Euler: velocity is updated by gravity
/// first, then position/orientation are updated using the *new* velocity
/// — more stable than explicit Euler for the same cost). Orientation is
/// integrated via `F::Bivector::exp` (rotor exponential map), not a naive
/// "add angle". Static bodies (`mass <= 0`) are untouched.
#[derive(Debug, Clone, Copy)]
pub struct Integrator<F: GaFlavor> {
    pub gravity: F::Vector,
}

impl<F: GaFlavor> Default for Integrator<F> {
    fn default() -> Self {
        Self {
            gravity: F::Vector::new(F::Scalar::ZERO, F::Scalar::from_f64(-9.81), F::Scalar::ZERO),
        }
    }
}

impl<F: GaFlavor> Integrator<F> {
    pub fn new(gravity: F::Vector) -> Self {
        Self { gravity }
    }

    pub fn step(&self, bodies: &mut [RigidBody<F>], dt: F::Scalar) {
        for body in bodies.iter_mut() {
            if body.inverse_mass() <= F::Scalar::ZERO {
                continue;
            }
            body.velocity = body.velocity + self.gravity * dt;
            // No torque source here yet (gravity acts at the center of
            // mass, so it never torques a body), so angular_velocity is
            // untouched by gravity — only ConstraintSolver changes it.
            let rotor_delta = (body.angular_velocity * dt).exp();
            let linear_delta = body.velocity * dt;
            let motor_delta = F::Motor::from_rotation_translation(rotor_delta, linear_delta);
            body.frame = body.frame.compose(motor_delta);
        }
    }
}
