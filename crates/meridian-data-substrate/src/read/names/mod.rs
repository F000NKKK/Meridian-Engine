//! The entity names a document is read with: its own keys, the ones below
//! its instances, and the resolver that applies them.

mod document_names;
mod document_resolver;
mod named;
mod pending_ref;

pub(crate) use document_names::DocumentNames;
pub(crate) use document_resolver::DocumentResolver;
pub(crate) use pending_ref::PendingRef;
