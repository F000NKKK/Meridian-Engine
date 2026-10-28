//! Reading a document (text or binary) into a draft, collecting every
//! problem first: all or nothing.

mod component_reader;
mod context;
mod document_reader;
mod document_role;
mod document_source;
mod finding;
mod names;
mod override_reader;
mod placement;
mod reading;

pub(crate) use context::ReadContext;
pub(crate) use document_role::DocumentRole;
pub(crate) use document_source::DocumentSource;
pub(crate) use finding::Finding;
pub(crate) use names::{DocumentNames, DocumentResolver};
