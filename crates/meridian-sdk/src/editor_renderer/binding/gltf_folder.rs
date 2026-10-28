//! [`GltfFolder`]: models read from a folder of glTF files.

use std::path::{Component, Path, PathBuf};

use meridian_data_substrate::AssetKey;
use meridian_rendering_domain::{MaterialHandle, MeshHandle};

use super::{AssetBinder, BuiltinAssets};
use crate::GraphicsSession;
use crate::model_loading::LoadedModel;

/// Answers a reference to `models/hero.glb` with the glTF file of that name
/// under a root folder, and everything else (the `builtin/...` primitives)
/// as [`BuiltinAssets`] does. The path is data from a file, so a key that
/// would leave the folder (`..`, an absolute path) or that is not a `.gltf`
/// or `.glb` names nothing. Loaded files are cached by the session, so a
/// model many entities refer to is read once.
#[derive(Debug)]
pub struct GltfFolder {
    root: PathBuf,
    builtin: BuiltinAssets,
}

impl GltfFolder {
    /// Models under `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            builtin: BuiltinAssets::default(),
        }
    }

    /// The file `key` names under the root, when it is a model file that stays inside.
    fn file_of(&self, key: &AssetKey) -> Option<PathBuf> {
        let relative = Path::new(key.as_str());
        let inside = relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)));
        let is_model = relative
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("gltf") || ext.eq_ignore_ascii_case("glb"));
        (inside && is_model).then(|| self.root.join(relative))
    }
}

impl AssetBinder for GltfFolder {
    fn mesh(&mut self, session: &mut GraphicsSession, key: &AssetKey) -> Option<MeshHandle> {
        self.builtin.mesh(session, key)
    }

    fn material(
        &mut self,
        session: &mut GraphicsSession,
        key: &AssetKey,
    ) -> Option<MaterialHandle> {
        self.builtin.material(session, key)
    }

    fn model(&mut self, session: &mut GraphicsSession, key: &AssetKey) -> Option<LoadedModel> {
        let file = self.file_of(key)?;
        session.load_model_gltf(file.to_str()?).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(path: &str) -> AssetKey {
        AssetKey::new(path).unwrap()
    }

    #[test]
    fn only_model_files_inside_the_folder_are_named() {
        let folder = GltfFolder::new("/assets");
        assert_eq!(
            folder.file_of(&key("models/hero.glb")),
            Some(PathBuf::from("/assets/models/hero.glb"))
        );
        assert!(folder.file_of(&key("models/hero.GLTF")).is_some());
        assert!(folder.file_of(&key("../secret.glb")).is_none());
        assert!(folder.file_of(&key("/etc/hero.glb")).is_none());
        assert!(folder.file_of(&key("models/hero.obj")).is_none());
        assert!(folder.file_of(&key("builtin/cube")).is_none());
    }
}
