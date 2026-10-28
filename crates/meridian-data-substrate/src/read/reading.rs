//! [`Reading`]: what reading a document came to.

use substrate_data::Diagnostic;

use super::finding::Finding;
use crate::draft::DocumentDraft;

/// What reading a document came to: the draft when nothing is wrong, and
/// every problem otherwise (Substrate Data's, then this crate's).
pub(crate) struct Reading {
    pub(crate) draft: Option<DocumentDraft>,
    pub(crate) problems: Vec<Diagnostic>,
    pub(crate) findings: Vec<Finding>,
}
