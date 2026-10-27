//! Convex volumes and the [`Shape`] support interface, generic over the flavor.

mod aabb;
mod cone;
mod convex_volume;
pub mod fixed;
pub mod float;
mod obb;
mod plane;
mod shape;
mod sphere;

pub use aabb::Aabb;
pub use cone::Cone;
pub use convex_volume::ConvexVolume;
pub use obb::Obb;
pub use plane::Plane;
pub use shape::Shape;
pub use sphere::Sphere;
