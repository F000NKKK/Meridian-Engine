//! [`Reading`]: what reading a document came to.

use substrate_data::Diagnostic;

use super::finding::Finding;
use super::names::PendingRef;
use crate::draft::DocumentDraft;

/// What reading a document came to: the draft when nothing is wrong, and
/// every problem otherwise (Substrate Data's, then this crate's).
pub(crate) struct Reading {
    pub(crate) draft: Option<DocumentDraft>,
    pub(crate) problems: Vec<Diagnostic>,
    pub(crate) findings: Vec<Finding>,
    /// References to entities a prefab is yet to make.
    pub(crate) pending: Vec<PendingRef>,
}
