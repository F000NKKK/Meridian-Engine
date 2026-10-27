//! Handle-addressed CPU-side registries for meshes and materials.

mod material_registry;
mod mesh_registry;
mod mesh_registry_error;
mod mesh_source;

pub use material_registry::MaterialRegistry;
pub use mesh_registry::MeshRegistry;
pub use mesh_registry_error::MeshRegistryError;
pub use mesh_source::MeshSource;
