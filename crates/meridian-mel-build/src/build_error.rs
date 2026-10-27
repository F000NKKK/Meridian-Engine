//! [`BuildError`]: why a `.mel` document could not be compiled.

use std::fmt;
use std::path::PathBuf;

use meridian_dsl_core::{ParseError, Span};

use crate::InvalidMarkup;

#[derive(Debug)]
pub enum BuildError {
    /// `OUT_DIR` is not set: not running inside a Cargo build script.
    OutDirMissing,
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
    Parse {
        path: PathBuf,
        error: ParseError,
    },
    /// Something the parser accepts but `.mel` code-behind does not.
    Invalid {
        path: PathBuf,
        span: Span,
        reason: InvalidMarkup,
    },
}

impl BuildError {
    pub(crate) fn invalid(path: &std::path::Path, span: Span, reason: InvalidMarkup) -> Self {
        Self::Invalid {
            path: path.to_path_buf(),
            span,
            reason,
        }
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutDirMissing => f.write_str("OUT_DIR is not set; call this from build.rs"),
            Self::Io { path, error } => write!(f, "{}: {error}", path.display()),
            Self::Parse { path, error } => write!(f, "{}:{error}", path.display()),
            Self::Invalid { path, span, reason } => {
                write!(f, "{}:{span}: {reason}", path.display())
            }
        }
    }
}

impl std::error::Error for BuildError {}
