//! Scene serialization on Substrate Data: an application-built
//! [`ComponentRegistry`], the `.mscene` document, and `World` load/save
//! with every problem typed and placed (line, column, path).
//!
//! Tier: above `ecs-core` and `gac-core` (whose `data` feature gives
//! `Motor3` its schema), below `engine-core`. It knows no subsystem's
//! components; an application registers the ones its scenes hold. Format,
//! decisions and status: docs/data-formats.md and
//! [ADR 016](../../../docs/adr/016-data-formats-on-substrate-data.md).

mod document_keys;
mod identity;
mod load;
mod mapping;
mod no_resolver;
mod registry;
mod scene_writer;
mod transform_form;

pub use document_keys::FORMAT;
pub use identity::{
    ComponentName, ComponentNameError, DisplayName, EntityKey, EntityKeyError, EntityKind,
    PrefabKind, PrefabPath, PrefabPathError,
};
pub use load::{LoadError, SceneIssue, SceneProblem};
pub use mapping::{DuplicateKey, KeptComponent, Parent, SceneEntity, SceneMap};
pub use no_resolver::NoResolver;
pub use registry::{
    ComponentEditError, ComponentRegistry, ComponentShape, RegisterError, UnknownComponents,
};
pub use transform_form::{TransformCodec, TransformForm};
