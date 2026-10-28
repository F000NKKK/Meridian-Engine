//! A document read without a problem and not spawned yet.

mod component_override;
mod components_draft;
mod document_draft;
mod override_edit;
mod pending_entity;

pub(crate) use component_override::ComponentOverride;
pub(crate) use components_draft::ComponentsDraft;
pub(crate) use document_draft::DocumentDraft;
pub(crate) use override_edit::OverrideEdit;
pub(crate) use pending_entity::PendingEntity;
