//! Contact generation for cuboids by the separating axis test.

use super::axis_x;
use super::axis_y;
use super::axis_z;
use meridian_gac_core::volume::Obb;
use meridian_gac_core::{GaFlavor, MotorLike, ScalarLike, Shape, VectorLike};

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
/// [`NarrowPhase::generate_contacts`](super::NarrowPhase::generate_contacts), the caller that divides it when
/// expanding one pair into multiple [`Contact`](super::Contact)s, so the sum of
/// per-point positional corrections reproduces the same total magnitude
/// a single-point contact would have applied instead of over-correcting
/// once per extra point).
type Manifold<F> = (
    Vec<<F as GaFlavor>::Vector>,
    <F as GaFlavor>::Scalar,
    <F as GaFlavor>::Vector,
);

/// The up-to-15 separating-axis candidates: both boxes' face normals and
/// the cross products of every edge pair.
fn candidate_axes<F: GaFlavor>(
    axes_a: &[F::Vector; 3],
    axes_b: &[F::Vector; 3],
) -> Vec<(F::Vector, SatAxisOwner)> {
    let mut candidates: Vec<(F::Vector, SatAxisOwner)> = Vec::with_capacity(15);
    for (i, &axis) in axes_a.iter().enumerate() {
        candidates.push((axis, SatAxisOwner::A(i)));
    }
    for (i, &axis) in axes_b.iter().enumerate() {
        candidates.push((axis, SatAxisOwner::B(i)));
    }
    for &ai in axes_a {
        for &bi in axes_b {
            let cross = ai.cross(bi);
            // Near-parallel edges: the cross product is ~zero and this
            // axis is redundant with the face-normal axes already tested.
            if cross.length() > F::Scalar::EPSILON {
                candidates.push((cross.normalize(), SatAxisOwner::Edge));
            }
        }
    }
    candidates
}

pub(crate) fn cuboid_vs_cuboid<F: GaFlavor>(obb_a: &Obb<F>, obb_b: &Obb<F>) -> Option<Manifold<F>> {
    let axes_a = obb_axes(obb_a);
    let axes_b = obb_axes(obb_b);
    let center_delta =
        obb_b.frame.transform_point(F::Vector::ZERO) - obb_a.frame.transform_point(F::Vector::ZERO);

    let mut min_overlap = F::Scalar::MAX;
    let mut best_axis = axis_x::<F>();
    let mut best_owner = SatAxisOwner::Edge;
    for (axis, owner) in candidate_axes::<F>(&axes_a, &axes_b) {
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
