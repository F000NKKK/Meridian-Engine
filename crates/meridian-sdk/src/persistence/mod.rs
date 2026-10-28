//! Scenes, prefabs and saves between files and a `World`.
//!
//! The application-facing wiring of `meridian-data-substrate` (which has no
//! filesystem): [`AssetPrefabSource`] is its `PrefabSource` port implemented
//! over the asset layer's [`AssetLoader`](meridian_asset_app::AssetLoader),
//! and [`WorldFiles`] loads and saves `.mscene` / `.mprefab` / `.msave` by
//! path. Format: docs/data-formats.md; layering: docs/notes/meridian-sdk.md.

mod asset_prefab_source;
mod raw_bytes;
mod scene_form;
mod world_files;
mod world_files_error;
mod write_file;

pub use asset_prefab_source::AssetPrefabSource;
// What an application needs beside `WorldFiles`: the data layer's registry,
// ids and prefab types, and the ecs types a scene holds.
pub use meridian_data_substrate::{
    ComponentName, ComponentRegistry, EntityKey, LoadedSave, NoResolver, PrefabInstance,
    PrefabLibrary, PrefabPath, PrefabSource, PrefabUnavailable, SaveHeader, SceneMap,
    SceneSaveError, SceneView, TransformCodec, UnknownComponents,
};
pub use meridian_ecs::{Component, Entity, Transform, World};
pub use scene_form::SceneForm;
pub use substrate_data::{Resolve, Schema};
pub use world_files::WorldFiles;
pub use world_files_error::WorldFilesError;
