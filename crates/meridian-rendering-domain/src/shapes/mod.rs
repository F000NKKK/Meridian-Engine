//! Procedural meshes: each builder returns a real [`MeshSource`](crate::submission::MeshSource)
//! (positions, normals, UVs, indices), all computed in code with no file I/O
//! (loading meshes from files is the application's job), plus the smooth
//! normals an imported mesh needs.

mod cube;
mod ground;
mod icosphere;
mod pyramid;
mod smooth_normals;

pub use cube::cube_mesh_source;
pub use ground::ground_mesh_source;
pub use icosphere::icosphere_mesh_source;
pub use pyramid::pyramid_mesh_source;
pub use smooth_normals::smooth_normals;
