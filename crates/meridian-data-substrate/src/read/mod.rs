//! Reading a document (text or binary) into a draft, collecting every
//! problem first: all or nothing.

mod context;
mod document_reader;
mod document_resolver;
mod document_role;
mod document_source;
mod finding;
mod placement;
mod reading;

pub(crate) use context::ReadContext;
pub(crate) use document_role::DocumentRole;
pub(crate) use document_source::DocumentSource;
pub(crate) use finding::Finding;
