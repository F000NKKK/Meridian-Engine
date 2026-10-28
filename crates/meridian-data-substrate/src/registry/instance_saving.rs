//! Saving prefab instances as what changed from their prefabs.

use substrate_data::{Schema, Value};

use super::ComponentRegistry;
use crate::document_kind::DocumentKind;
use crate::save::{SaveHeader, write_envelope};
use crate::scene_writer::SceneWriter;
use crate::{PrefabSource, SceneSaveError, SceneView};

impl ComponentRegistry {
    /// `view` as `.mscene` text, each prefab instance written as the
    /// changes from its prefab rather than as all its components: the
    /// prefab path, and `overrides` holding, keyed by the entity's key
    /// (`hero` for the instance, `hero/gun` for what the prefab made), the
    /// fields that differ from the prefab, the components added whole, and
    /// the ones removed. An instance nothing changed in has no `overrides`.
    /// Loading what this writes, with the same prefabs, gives the same world.
    ///
    /// [`ComponentRegistry::save_scene`] writes the instance with all its
    /// current components instead and needs no prefabs, but cannot save an
    /// edit to an entity the prefab made.
    ///
    /// # Errors
    ///
    /// [`SceneSaveError`] when the prefab of an instance can't be read.
    pub fn save_scene_with_prefabs(
        &self,
        view: SceneView<'_>,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<String, SceneSaveError> {
        Ok(self
            .instance_writer(view, prefabs)?
            .text(DocumentKind::Scene))
    }

    /// The same document in the `SDAT` binary form.
    ///
    /// # Errors
    ///
    /// [`SceneSaveError`], as [`ComponentRegistry::save_scene_with_prefabs`].
    pub fn save_scene_binary_with_prefabs(
        &self,
        view: SceneView<'_>,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<Vec<u8>, SceneSaveError> {
        Ok(self
            .instance_writer(view, prefabs)?
            .bytes(DocumentKind::Scene))
    }

    /// The scene document of [`ComponentRegistry::save_scene_with_prefabs`]
    /// as a value.
    ///
    /// # Errors
    ///
    /// [`SceneSaveError`], as [`ComponentRegistry::save_scene_with_prefabs`].
    pub fn scene_document_with_prefabs(
        &self,
        view: SceneView<'_>,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<Value, SceneSaveError> {
        Ok(self
            .instance_writer(view, prefabs)?
            .document(DocumentKind::Scene))
    }

    /// A save as [`ComponentRegistry::save_game`] writes it, with the
    /// scene's prefab instances written as changes from their prefabs
    /// ([`ComponentRegistry::save_scene_with_prefabs`]); loading it needs
    /// the same prefabs.
    ///
    /// # Errors
    ///
    /// [`SceneSaveError`] when the prefab of an instance can't be read.
    pub fn save_game_with_prefabs<T: Schema>(
        &self,
        header: &SaveHeader,
        view: SceneView<'_>,
        state: &T,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<Vec<u8>, SceneSaveError> {
        let document = self.scene_document_with_prefabs(view, prefabs)?;
        Ok(write_envelope(header, &document, state))
    }

    fn instance_writer<'a>(
        &'a self,
        view: SceneView<'a>,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<SceneWriter<'a>, SceneSaveError> {
        SceneWriter::new(self, view.world, view.scene).against_prefabs(prefabs)
    }
}
