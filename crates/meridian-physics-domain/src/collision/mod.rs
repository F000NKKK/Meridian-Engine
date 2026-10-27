//! Collision detection written once, generic over `F: GaFlavor`: [`BroadPhase`],
//! [`NarrowPhase`], [`Contact`], [`ColliderShape`] and the sphere/cuboid contact
//! generators. `float` and `fixed` bind it to the two flavors.
//!
//! Nothing here has a GPU-dispatch constraint of its own, so unlike
//! `gac`'s `Motor3`/`Vec3`, the engine is not duplicated per flavor; the
//! flavor modules are thin aliases (see CLAUDE.md, "Float/Fixed branching").

mod axes;
mod broad_phase;
mod collider_mesh_handle;
mod collider_shape;
mod contact;
mod cuboid_contact;
pub mod fixed;
pub mod float;
mod narrow_phase;
mod sphere_contact;

pub(crate) use axes::{axis_x, axis_y, axis_z};
pub use broad_phase::BroadPhase;
pub(crate) use broad_phase::aabb_of;
pub use collider_mesh_handle::{ColliderMeshHandle, ColliderMeshMarker};
pub use collider_shape::ColliderShape;
pub use contact::Contact;
pub(crate) use cuboid_contact::cuboid_vs_cuboid;
pub use narrow_phase::NarrowPhase;
pub(crate) use sphere_contact::{sphere_vs_cuboid, sphere_vs_sphere};
