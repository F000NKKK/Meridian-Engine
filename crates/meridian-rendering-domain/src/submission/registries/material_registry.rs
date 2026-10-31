//! Registered materials by handle.

use meridian_memory::ResourcePool;

use crate::MaterialHandle;
use crate::scene::Material;

/// Handle-addressed storage for [`Material`]s. Materials are plain data
/// (see `scene::Material`'s own doc comment); this registry exists so
/// [`Renderable3d::material`](crate::scene::Renderable3d::material) is a stable handle rather than an inline
/// value, matching every other resource identity in the workspace.
#[derive(Debug, Default)]
pub struct MaterialRegistry {
    materials: ResourcePool<Material>,
}

impl MaterialRegistry {
    /// Creates an empty material registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a material and returns its stable handle.
    pub fn register(&mut self, material: Material) -> MaterialHandle {
        MaterialHandle::new(self.materials.insert(material))
    }

    /// Removes and returns the material for `handle`, if still live.
    pub fn unregister(&mut self, handle: MaterialHandle) -> Option<Material> {
        self.materials.remove(handle.handle)
    }

    /// Returns the material for a live handle.
    pub fn get(&self, handle: MaterialHandle) -> Option<&Material> {
        self.materials.get(handle.handle)
    }
}
