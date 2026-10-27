//! Contact generation for spheres.

use super::axis_x;
use meridian_gac_core::generic::{GaFlavor, MotorLike, Obb, ScalarLike, VectorLike};

/// Sphere-sphere exact test. `None` if not overlapping.
pub(crate) fn sphere_vs_sphere<F: GaFlavor>(
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
        closest_from_inside(obb, local)
    } else {
        closest_from_outside(obb, local, point)
    }
}

/// [`closest_point_on_obb`] for a `local`-space point already inside the
/// box: pushes out through the nearest face.
fn closest_from_inside<F: GaFlavor>(
    obb: &Obb<F>,
    local: F::Vector,
) -> (F::Vector, F::Vector, F::Scalar) {
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
        face_or_local(local_normal.x(), obb.half_extents.x(), local.x()),
        face_or_local(local_normal.y(), obb.half_extents.y(), local.y()),
        face_or_local(local_normal.z(), obb.half_extents.z(), local.z()),
    );
    let closest_world = obb.frame.transform_point(closest_local);
    (
        closest_world,
        obb.frame.transform_vector(local_normal),
        -depth,
    )
}

/// The face coordinate along an axis the push-out normal points along,
/// otherwise the point's own coordinate.
fn face_or_local<S: ScalarLike>(normal_component: S, half_extent: S, local: S) -> S {
    if normal_component == S::ZERO {
        local
    } else {
        half_extent * normal_component
    }
}

/// [`closest_point_on_obb`] for a point outside the box: clamping its
/// `local` coordinates to the box's extents.
fn closest_from_outside<F: GaFlavor>(
    obb: &Obb<F>,
    local: F::Vector,
    point: F::Vector,
) -> (F::Vector, F::Vector, F::Scalar) {
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

/// Sphere-cuboid exact test via [`closest_point_on_obb`]. Returns the
/// contact point, outward normal (cuboid toward sphere), and penetration.
pub(crate) fn sphere_vs_cuboid<F: GaFlavor>(
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
