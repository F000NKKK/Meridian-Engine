//! [`ResourceError`]: a [`ResourceProblem`] placed in its file.

use std::path::{Path, PathBuf};

use super::ResourceProblem;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}:{line}:{column}: {problem}", path.display())]
pub struct ResourceError {
    pub path: PathBuf,
    pub line: u32,
    pub column: u32,
    pub problem: ResourceProblem,
}

impl ResourceError {
    pub(crate) fn at(path: &Path, text: &str, offset: usize, problem: ResourceProblem) -> Self {
        let at = substrate_data::text::LineCol::of(text, offset);
        Self {
            path: path.to_path_buf(),
            line: at.line,
            column: at.col,
            problem,
        }
    }
}
