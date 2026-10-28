//! Writing an instance's changes as `overrides`: its prefab is read again
//! and each entity it made is compared with the world's.

mod component_docs;
mod instance_origin;
mod scene_save_error;
mod target_changes;

pub(crate) use component_docs::ComponentDocs;
pub(crate) use instance_origin::InstanceOrigin;
pub use scene_save_error::SceneSaveError;
pub(crate) use target_changes::TargetChanges;
