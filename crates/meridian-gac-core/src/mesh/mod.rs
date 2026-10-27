//! Indexed polygon meshes and procedural generation.

mod face;
pub mod fixed;
pub mod float;
mod icosphere;
mod indexed_mesh;

pub use face::Face;
pub use icosphere::icosphere;
pub use indexed_mesh::Mesh;
