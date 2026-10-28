//! [`MeshReference`]: the mesh and material a drawable entity names in a file.

use meridian_ecs::Component;
use substrate_data::{Id, Schema};

use crate::AssetPathKind;

/// What a scene file says about a drawable entity: the mesh and the material
/// it uses, as `@asset:` references (`mesh_renderer: { mesh: "@asset:meshes/hero.obj", ... }`).
/// It is the authored form; the sdk resolves it into the runtime
/// `MeshRenderer` handles once the assets are loaded, and saving writes this
/// component back, never a path guessed from a handle.
#[derive(Debug, Clone, PartialEq, Eq, Schema)]
pub struct MeshReference {
    /// The mesh asset.
    pub mesh: Id<AssetPathKind>,
    /// The material asset.
    pub material: Id<AssetPathKind>,
    /// Whether the mesh always faces the camera.
    #[data(default)]
    pub billboard: bool,
}

impl Component for MeshReference {}
