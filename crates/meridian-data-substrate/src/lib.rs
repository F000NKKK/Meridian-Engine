//! Scene serialization on Substrate Data: an application-built
//! [`ComponentRegistry`] and three formats over a `World`, every problem
//! typed and placed. Format and status: docs/data-formats.md.
//!
//! - **`.mscene`**: text ([`ComponentRegistry::load_scene`]; problems with
//!   path, line and column) or the `SDAT` binary form
//!   ([`load_scene_binary`](ComponentRegistry::load_scene_binary); path, and
//!   a byte offset for bytes that do not decode);
//!   [`load_scene_bytes`](ComponentRegistry::load_scene_bytes) tells them
//!   apart by magic. All or nothing, into a [`SceneMap`].
//! - **`.mprefab`**: a scene entity that says `prefab` becomes the
//!   instance root, the prefab's others are `<instance>/<prefab id>`
//!   ([`load_scene_with_prefabs`](ComponentRegistry::load_scene_with_prefabs)).
//!   Bytes come from the application's [`PrefabSource`]; no files are read.
//!   An instance's `overrides` change single fields of the entities its
//!   prefab makes (keyed `<instance>/<prefab id>`), and
//!   [`save_scene_with_prefabs`](ComponentRegistry::save_scene_with_prefabs)
//!   writes an instance as only what changed from its prefab.
//! - **`.msave`**: `{ format, game, created, scene, state }`: the full
//!   scene document and the game's own `state`
//!   ([`save_game`](ComponentRegistry::save_game),
//!   [`load_game`](ComponentRegistry::load_game)); [`SaveHeader::read`]
//!   does not decode the scene or the state.
//!
//! A world's disk form is its scene document; `World::snapshot` in `ecs`
//! is the in-memory copy and is never written, and no runtime `Entity`
//! reaches a file. Tier: above `ecs` and `gac`, below `runtime-app`; it
//! knows no subsystem's components. See
//! [ADR 016](../../../docs/adr/016-data-formats-on-substrate-data.md).

mod document_keys;
mod document_kind;
mod draft;
mod identity;
mod load;
mod mapping;
mod no_resolver;
mod overrides;
mod patch;
mod prefab;
mod read;
mod registry;
mod rendering;
mod save;
mod scene_view;
mod scene_writer;
mod transform_form;

pub use document_keys::{FORMAT, SAVE_FORMAT};
pub use identity::{
    AssetKey, AssetKeyError, AssetPathKind, ComponentName, ComponentNameError, DisplayName,
    EntityKey, EntityKeyError, EntityKind, PrefabKind, PrefabPath, PrefabPathError,
};
pub use load::{LoadError, SceneIssue, SceneProblem};
pub use mapping::{DuplicateKey, KeptComponent, Parent, SceneEntity, SceneMap};
pub use no_resolver::NoResolver;
pub use overrides::SceneSaveError;
pub use prefab::{
    MAX_PREFAB_DEPTH, PrefabInstance, PrefabLibrary, PrefabSource, PrefabUnavailable,
};
pub use registry::{
    ComponentEditError, ComponentRegistry, ComponentShape, RegisterError, UnknownComponents,
};
pub use rendering::{LightCodec, LightForm, LightKindForm, MeshReference, register_rendering};
pub use save::{LoadedSave, SaveError, SaveHeader};
pub use scene_view::SceneView;
pub use transform_form::{TransformCodec, TransformForm};
