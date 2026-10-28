//! [`WorldFiles`]: scene, prefab and save files by path.

use std::path::PathBuf;

use meridian_asset_app::{AssetLoader, AssetPath, LoadError};
use meridian_data_substrate::{
    ComponentRegistry, LoadedSave, PrefabInstance, SaveHeader, SceneMap,
};
use meridian_ecs::World;
use substrate_data::{Resolve, Schema};

use super::raw_bytes::RawBytes;
use super::write_file::write_file;
use super::{AssetPrefabSource, SceneForm, SceneView, WorldFilesError};
use crate::default_asset_loader;

/// Loads and saves `.mscene`, `.mprefab` and `.msave` files into a `World`.
///
/// Built by the application (no global) from the [`ComponentRegistry`] it
/// registered its components in, and a directory: paths are relative to it
/// and may not leave it. Reads run on the asset loader's background threads
/// (ADR 027) and the calling thread waits, so call these from a loading
/// thread, never from a frame; a prefab a scene instances is read the same
/// way, with `<path>.mprefab` as its file ([`AssetPrefabSource`]). Every
/// load is all or nothing. A world's disk form is its scene document, never
/// `World::snapshot` (docs/data-formats.md).
#[derive(Debug)]
pub struct WorldFiles {
    root: PathBuf,
    loader: AssetLoader,
    registry: ComponentRegistry,
}

impl WorldFiles {
    /// Files under `root`, read through `loader` (which must read the same
    /// directory), with `registry` naming the components.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, loader: AssetLoader, registry: ComponentRegistry) -> Self {
        Self {
            root: root.into(),
            loader,
            registry,
        }
    }

    /// Files under `root`, read on the default background pool
    /// ([`default_asset_loader`]).
    #[must_use]
    pub fn in_directory(root: impl Into<PathBuf>, registry: ComponentRegistry) -> Self {
        let root = root.into();
        let loader = default_asset_loader(root.clone());
        Self::new(root, loader, registry)
    }

    /// The registry, for `describe()` and component editing.
    #[must_use]
    pub fn registry(&self) -> &ComponentRegistry {
        &self.registry
    }

    /// Loads the scene at `path` (text or binary, by magic) into `world`,
    /// instancing the prefabs it names. `resolver` answers the references
    /// that are not `@entity:` or `@prefab:` ones (`NoResolver` keeps them as
    /// written).
    ///
    /// # Errors
    ///
    /// [`WorldFilesError::Read`] if the file is missing, or
    /// [`WorldFilesError::Scene`] with every problem in the scene or in a
    /// prefab (a missing prefab is `PrefabUnavailable` at its entity).
    pub fn load_scene(
        &self,
        path: &str,
        world: &mut World,
        resolver: &mut dyn Resolve,
    ) -> Result<SceneMap, WorldFilesError> {
        let bytes = self.read(path)?;
        let mut prefabs = self.prefabs();
        Ok(self
            .registry
            .load_scene_with_prefabs(&bytes, world, resolver, &mut prefabs)?)
    }

    /// Spawns the prefab file at `prefab` (a `PrefabPath`, `actors/hero`) as
    /// the entity `instance` names, from code rather than from a scene.
    ///
    /// # Errors
    ///
    /// [`WorldFilesError::Scene`], as [`load_scene`](Self::load_scene).
    pub fn spawn_prefab(
        &self,
        instance: &PrefabInstance,
        world: &mut World,
        resolver: &mut dyn Resolve,
    ) -> Result<SceneMap, WorldFilesError> {
        let mut prefabs = self.prefabs();
        Ok(self
            .registry
            .instantiate_prefab(instance, world, resolver, &mut prefabs)?)
    }

    /// Writes the entities of `view` as a scene document at `path`. A prefab
    /// instance is written as what changed from its prefab (`overrides`),
    /// including edits to the entities the prefab made, so each instance's
    /// prefab is read again from the directory.
    ///
    /// # Errors
    ///
    /// [`WorldFilesError::Instance`] when an instance's prefab can't be read,
    /// [`WorldFilesError::PathRejected`] or [`WorldFilesError::Write`].
    pub fn save_scene(
        &self,
        path: &str,
        view: SceneView<'_>,
        form: SceneForm,
    ) -> Result<(), WorldFilesError> {
        let mut prefabs = self.prefabs();
        let bytes = match form {
            SceneForm::Text => self
                .registry
                .save_scene_with_prefabs(view, &mut prefabs)?
                .into_bytes(),
            SceneForm::Binary => self
                .registry
                .save_scene_binary_with_prefabs(view, &mut prefabs)?,
        };
        write_file(&self.root, path, &bytes)
    }

    /// Loads the save at `path` into `world`: its scene (with prefabs) and
    /// the game's `state`.
    ///
    /// # Errors
    ///
    /// [`WorldFilesError::Read`] or [`WorldFilesError::Save`] (malformed,
    /// another `format`, a scene or state that does not read).
    pub fn load_game<T: Schema>(
        &self,
        path: &str,
        world: &mut World,
        resolver: &mut dyn Resolve,
    ) -> Result<LoadedSave<T>, WorldFilesError> {
        let bytes = self.read(path)?;
        let mut prefabs = self.prefabs();
        Ok(self
            .registry
            .load_game(&bytes, world, resolver, &mut prefabs)?)
    }

    /// Writes a save at `path`: `header`, the entities of `view` and the
    /// game's `state`. Prefab instances are written as in
    /// [`save_scene`](Self::save_scene).
    ///
    /// # Errors
    ///
    /// [`WorldFilesError::Instance`], [`WorldFilesError::PathRejected`] or
    /// [`WorldFilesError::Write`].
    pub fn save_game<T: Schema>(
        &self,
        path: &str,
        header: &SaveHeader,
        view: SceneView<'_>,
        state: &T,
    ) -> Result<(), WorldFilesError> {
        let mut prefabs = self.prefabs();
        let bytes = self
            .registry
            .save_game_with_prefabs(header, view, state, &mut prefabs)?;
        write_file(&self.root, path, &bytes)
    }

    /// The header of the save at `path`, without decoding its scene or state.
    ///
    /// # Errors
    ///
    /// [`WorldFilesError::Read`] or [`WorldFilesError::Save`].
    pub fn read_save_header(&self, path: &str) -> Result<SaveHeader, WorldFilesError> {
        Ok(SaveHeader::read(&self.read(path)?)?)
    }

    fn prefabs(&self) -> AssetPrefabSource {
        AssetPrefabSource::new(self.loader.clone())
    }

    fn read(&self, path: &str) -> Result<Vec<u8>, WorldFilesError> {
        match self.loader.load(AssetPath::new(path), RawBytes).wait() {
            Ok(bytes) => Ok(bytes),
            Err(LoadError::Source(error)) => Err(WorldFilesError::Read(error)),
            Err(LoadError::Decode(never)) => match never {},
            Err(LoadError::Panicked | LoadError::Cancelled) => Err(WorldFilesError::Interrupted {
                path: path.to_owned(),
            }),
        }
    }
}
