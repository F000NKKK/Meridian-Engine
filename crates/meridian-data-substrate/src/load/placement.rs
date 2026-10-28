//! [`Placement`]: how a problem is put where it is in the document it
//! was found in.

use substrate_data::{Diagnostic, Doc};

use super::document_draft::DocumentDraft;
use super::finding::Finding;
use super::reading::Reading;
use super::{LoadError, SceneIssue};

/// How problems are placed: in text, by line and column; in binary there
/// are neither, so a problem keeps its path (and a decode error its byte
/// offset, see [`SceneIssue::offset`]).
pub(crate) enum Placement<'a> {
    /// A text document, and the `Doc` parsed from it.
    Text(&'a str, &'a Doc),
    /// A binary document.
    Binary,
}

impl Placement<'_> {
    /// The draft of a [`Reading`] that found nothing wrong, else the error
    /// with every problem, placed.
    pub(crate) fn draft(&self, reading: Reading) -> Result<DocumentDraft, LoadError> {
        let Reading {
            draft,
            problems,
            findings,
        } = reading;
        if let Some(draft) = draft {
            return Ok(draft);
        }
        let mut issues: Vec<SceneIssue> =
            problems.into_iter().map(|d| self.diagnostic(d)).collect();
        issues.extend(findings.into_iter().map(|f| self.finding(f)));
        Err(LoadError::new(issues))
    }

    fn diagnostic(&self, d: Diagnostic) -> SceneIssue {
        match self {
            Placement::Text(text, doc) => {
                SceneIssue::of_diagnostic(substrate_data::placed(text, doc, d))
            }
            Placement::Binary => SceneIssue::of_diagnostic(d),
        }
    }

    fn finding(&self, f: Finding) -> SceneIssue {
        match self {
            Placement::Text(text, doc) => {
                SceneIssue::placed(f.problem, f.path, f.on_key, (text, doc))
            }
            Placement::Binary => SceneIssue::unplaced(f.problem, f.path),
        }
    }
}
