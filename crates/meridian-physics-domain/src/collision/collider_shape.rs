//! The shape a rigid body collides with.

use meridian_gac::{GaFlavor, ScalarLike};

/// A collision shape. `Sphere` and `Cuboid` (`gac::Obb`'s
/// half-extents, oriented by the owning `RigidBody`'s own `frame` — no
/// separate orientation to keep in sync) so far; capsule/mesh (via
/// [`ColliderMeshHandle`](crate::collision::ColliderMeshHandle)) are additive later.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColliderShape<F: GaFlavor> {
    /// A sphere centered on the body's origin.
    Sphere {
        /// Radius in world units.
        radius: F::Scalar,
    },
    /// A box centered on and oriented by the body's frame.
    Cuboid {
        /// Half the box's size along each local axis.
        half_extents: F::Vector,
    },
}

impl<F: GaFlavor> Default for ColliderShape<F> {
    fn default() -> Self {
        ColliderShape::Sphere {
            radius: F::Scalar::from_f64(0.5),
        }
    }
}
