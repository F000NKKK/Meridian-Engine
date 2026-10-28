//! [`BuiltinAssets`]: the primitives the engine itself provides.

use std::collections::HashMap;

use meridian_data_substrate::AssetKey;
use meridian_rendering_domain::{
    Material, MaterialHandle, MeshHandle, cube_mesh_source, ground_mesh_source,
    icosphere_mesh_source,
};

use super::AssetBinder;
use crate::GraphicsSession;

/// Meshes `builtin/cube`, `builtin/sphere` and `builtin/ground`, and the
/// material `builtin/default`: what `@asset:builtin/...` names. Each is
/// registered once, on first use.
#[derive(Debug, Default)]
pub struct BuiltinAssets {
    meshes: HashMap<String, MeshHandle>,
    default_material: Option<MaterialHandle>,
}

impl AssetBinder for BuiltinAssets {
    fn mesh(&mut self, session: &mut GraphicsSession, key: &AssetKey) -> Option<MeshHandle> {
        let name = key.as_str();
        if let Some(handle) = self.meshes.get(name) {
            return Some(*handle);
        }
        let source = match name {
            "builtin/cube" => cube_mesh_source(0.5),
            "builtin/sphere" => icosphere_mesh_source(3, 0.5),
            "builtin/ground" => ground_mesh_source(10.0, 10.0),
            _ => return None,
        };
        let handle = session.render.meshes.register(source).ok()?;
        self.meshes.insert(name.to_owned(), handle);
        Some(handle)
    }

    fn material(
        &mut self,
        session: &mut GraphicsSession,
        key: &AssetKey,
    ) -> Option<MaterialHandle> {
        if key.as_str() != "builtin/default" {
            return None;
        }
        Some(
            *self
                .default_material
                .get_or_insert_with(|| session.render.materials.register(Material::default())),
        )
    }
}
