//! [`Finding`]: a problem this crate finds in a document.

use substrate_data::{Code, Diagnostic, Path, Problem};

use crate::load::SceneProblem;

/// A problem found here, where it is, and whether at its key.
pub(crate) struct Finding {
    pub(crate) problem: SceneProblem,
    pub(crate) path: Path,
    pub(crate) on_key: bool,
}

impl Finding {
    /// A Substrate Data diagnostic as a finding, where it says it is.
    pub(crate) fn of_diagnostic(d: Diagnostic) -> Self {
        let problem = match d.code {
            Some(Code::Data(p)) => p,
            _ => Problem::Invalid,
        };
        Finding {
            problem: SceneProblem::Data {
                problem,
                message: d.message,
            },
            path: d.path,
            on_key: d.on_key,
        }
    }
}
