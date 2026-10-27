//! `MeshRegistry` as a [`MeshStore`].

use meridian_graphics_core::{MeshHandle, MeshRegistry, MeshRegistryError, MeshSource};

use crate::MeshStore;

impl MeshStore for MeshRegistry {
    fn add(&mut self, source: MeshSource) -> Result<MeshHandle, MeshRegistryError> {
        self.register(source)
    }

    fn get(&self, handle: MeshHandle) -> Option<&MeshSource> {
        MeshRegistry::get(self, handle)
    }

    fn remove(&mut self, handle: MeshHandle) -> Option<MeshSource> {
        self.unregister(handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::run_mesh_store_contract;

    #[test]
    fn the_registry_passes_the_mesh_store_contract() {
        run_mesh_store_contract(MeshRegistry::new);
    }
}
