//! [`SceneIssue`]: a problem and where it is.

use std::fmt;

use substrate_data::{Code, Diagnostic, Doc, Path, Problem, placed};

use super::SceneProblem;

/// A [`SceneProblem`] and where it is: the value's path in the document,
/// and its line and column (1-based) when the text is known.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneIssue {
    /// What's wrong.
    pub problem: SceneProblem,
    /// The value it's about (`entities[1].components.transform`).
    pub path: Path,
    /// Its line.
    pub line: Option<u32>,
    /// Its column.
    pub column: Option<u32>,
}

impl SceneIssue {
    /// A Substrate Data diagnostic, already placed.
    pub(crate) fn of_diagnostic(d: Diagnostic) -> Self {
        let problem = match d.code {
            Some(Code::Data(p)) => p,
            _ => Problem::Invalid,
        };
        let problem = SceneProblem::Data {
            problem,
            message: d.message,
        };
        SceneIssue {
            problem,
            path: d.path,
            line: d.line,
            column: d.col,
        }
    }

    /// `problem` at `path` of `doc` (parsed from `text`): at its key when
    /// `on_key`, else at its value.
    pub(crate) fn placed(
        problem: SceneProblem,
        path: Path,
        on_key: bool,
        source: (&str, &Doc),
    ) -> Self {
        let d = placed(
            source.0,
            source.1,
            Diagnostic {
                path,
                on_key,
                ..Diagnostic::default()
            },
        );
        SceneIssue {
            problem,
            path: d.path,
            line: d.line,
            column: d.col,
        }
    }
}

impl fmt::Display for SceneIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (Some(line), Some(column)) = (self.line, self.column) {
            write!(f, "{line}:{column}: ")?;
        }
        if !self.path.is_root() {
            write!(f, "{}: ", self.path)?;
        }
        self.problem.fmt(f)
    }
}
