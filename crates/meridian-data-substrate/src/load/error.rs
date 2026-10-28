//! [`LoadError`]: why a scene didn't load.

use std::fmt;

use substrate_data::{Diagnostic, SyntaxError};

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
