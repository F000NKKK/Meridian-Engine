//! [`LoadError`]: why a scene didn't load.

use std::fmt;

use substrate_data::binary::DecodeError;
use substrate_data::{Diagnostic, Problem, SyntaxError};

use super::SceneIssue;

/// Why a scene didn't load: every problem in the document, in file order.
/// Nothing was spawned.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadError {
    issues: Vec<SceneIssue>,
}

impl LoadError {
    /// `issues`, put in file order.
    pub(crate) fn new(mut issues: Vec<SceneIssue>) -> Self {
        issues.sort_by_key(|i| (i.line, i.column));
        LoadError { issues }
    }

    /// Text that doesn't parse.
    pub(crate) fn syntax(e: SyntaxError) -> Self {
        LoadError::new(vec![SceneIssue::of_diagnostic(Diagnostic::syntax(e))])
    }

    /// Bytes that are not the binary form, at the byte that shows it.
    pub(crate) fn decode(e: DecodeError) -> Self {
        let mut issue =
            SceneIssue::of_diagnostic(Diagnostic::error(Problem::BadBinary, &e.message));
        issue.offset = Some(e.offset);
        LoadError::new(vec![issue])
    }

    /// Bytes that are neither the binary form nor UTF-8 text, valid up to `offset`.
    pub(crate) fn not_utf8(offset: usize) -> Self {
        let d = Diagnostic::error(Problem::Syntax, "not an SDAT document and not UTF-8 text");
        let mut issue = SceneIssue::of_diagnostic(d);
        issue.offset = Some(offset);
        LoadError::new(vec![issue])
    }

    /// Every problem, in file order.
    #[must_use]
    pub fn issues(&self) -> &[SceneIssue] {
        &self.issues
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, issue) in self.issues.iter().enumerate() {
            if i > 0 {
                f.write_str("\n")?;
            }
            issue.fmt(f)?;
        }
        Ok(())
    }
}

impl std::error::Error for LoadError {}
