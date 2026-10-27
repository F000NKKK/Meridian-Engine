//! Handle-addressed CPU-side registries for meshes, materials and textures.

mod material_registry;
mod mesh_registry;
mod mesh_registry_error;
mod mesh_source;
mod texture_registry;

pub use material_registry::MaterialRegistry;
pub use mesh_registry::MeshRegistry;
pub use mesh_registry_error::MeshRegistryError;
pub use mesh_source::MeshSource;
pub use texture_registry::TextureRegistry;
