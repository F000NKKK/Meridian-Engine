//! Saves (`.msave`) in and out of a [`ComponentRegistry`].

use meridian_ecs::World;
use substrate_data::{Resolve, Schema};

use super::ComponentRegistry;
use crate::document_kind::DocumentKind;
use crate::draft::DocumentDraft;
use crate::read::{DocumentRole, DocumentSource, ReadContext};
use crate::save::{
    LoadedSave, SaveError, SaveHeader, read_header_and_scene, read_state, write_envelope,
};
use crate::scene_writer::SceneWriter;
use crate::{PrefabSource, SceneMap};

impl ComponentRegistry {
    /// A save: the envelope `{ format, game, created, scene, state }` in the
    /// binary form. `scene` is the full scene document of `world`'s
    /// entities listed in `scene` (not a delta against the level file), and
    /// `state` is the game's own, written by its schema. The format written
    /// is always [`crate::SAVE_FORMAT`]; `header.created` is what the
    /// caller says (there is no clock here).
    #[must_use]
    pub fn save_game<T: Schema>(
        &self,
        header: &SaveHeader,
        world: &World,
        scene: &SceneMap,
        state: &T,
    ) -> Vec<u8> {
        let document = SceneWriter::new(self, world, scene).document(DocumentKind::Scene);
        write_envelope(header, &document, state)
    }

    /// Reads a save: its scene is spawned into `world` as a fresh set of
    /// entities (document keys are remapped, runtime `Entity` values never
    /// are in a file), prefabs instanced from `prefabs`, and the game's
    /// state is read by `T`'s schema. All or nothing: the state and the
    /// scene are both read before anything is spawned.
    ///
    /// # Errors
    ///
    /// [`SaveError`]: bytes that are not a save, a `format` this doesn't
    /// read, a scene that doesn't load, or a state `T` doesn't read.
    pub fn load_game<T: Schema>(
        &self,
        bytes: &[u8],
        world: &mut World,
        resolver: &mut dyn Resolve,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<LoadedSave<T>, SaveError> {
        let (header, scene) = self.read_save_scene(bytes, resolver, prefabs)?;
        let state = read_state::<T>(bytes)?;
        Ok(LoadedSave {
            header,
            scene: scene.spawn(world),
            state,
        })
    }

    /// Reads a save's header and spawns its scene, without decoding the
    /// game's state (the bytes after the scene are not looked at): what an
    /// editor or a save browser does with a game's save it has no schema for.
    ///
    /// # Errors
    ///
    /// [`SaveError`], as [`ComponentRegistry::load_game`] except for the state.
    pub fn load_game_scene(
        &self,
        bytes: &[u8],
        world: &mut World,
        resolver: &mut dyn Resolve,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<(SaveHeader, SceneMap), SaveError> {
        let (header, scene) = self.read_save_scene(bytes, resolver, prefabs)?;
        Ok((header, scene.spawn(world)))
    }

    fn read_save_scene(
        &self,
        bytes: &[u8],
        resolver: &mut dyn Resolve,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<(SaveHeader, DocumentDraft), SaveError> {
        let (header, scene) = read_header_and_scene(bytes, true)?;
        let scene = scene.ok_or(SaveError::MissingField { field: "scene" })?;
        let mut cx = ReadContext::new(self, resolver).instancing(prefabs);
        let draft = DocumentSource::Document(&scene)
            .draft(&DocumentRole::Scene, &mut cx)
            .map_err(SaveError::Scene)?;
        Ok((header, draft))
    }
}
