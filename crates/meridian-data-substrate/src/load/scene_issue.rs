//! [`SceneIssue`]: a problem and where it is.

use std::fmt;

use substrate_data::{Code, Diagnostic, Doc, Path, Problem, placed};

use super::SceneProblem;

/// A [`SceneProblem`] and where it is: the value's path in the document,
/// and its line and column (1-based) when the text is known. A binary
/// document has no lines: its problems have a path, and bytes that do not
/// decode have the byte `offset` too.
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
    /// The byte it was found at, for a binary document that does not
    /// decode. Substrate Data tracks no offsets per value, so a problem
    /// with a decoded value has its `path` only.
    pub offset: Option<usize>,
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
            offset: None,
        }
    }

    /// `problem` at `path` of a binary document: no line, column or offset.
    pub(crate) fn unplaced(problem: SceneProblem, path: Path) -> Self {
        SceneIssue {
            problem,
            path,
            line: None,
            column: None,
            offset: None,
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
            offset: None,
        }
    }
}

impl fmt::Display for SceneIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (Some(line), Some(column)) = (self.line, self.column) {
            write!(f, "{line}:{column}: ")?;
        }
        if let Some(offset) = self.offset {
            write!(f, "byte {offset}: ")?;
        }
        if !self.path.is_root() {
            write!(f, "{}: ", self.path)?;
        }
        self.problem.fmt(f)
    }
}
