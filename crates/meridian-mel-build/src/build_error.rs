//! [`BuildError`]: why a `.mel` document could not be compiled.

use std::path::{Path, PathBuf};

use meridian_dsl_core::{ParseError, Span};

use crate::InvalidMarkup;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// `OUT_DIR` is not set: not running inside a Cargo build script.
    #[error("OUT_DIR is not set; call this from build.rs")]
    OutDirMissing,
    #[error("{}: {error}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        error: std::io::Error,
    },
    /// A file found under the compiled directory is not inside it (the
    /// directory walk followed a link elsewhere), so it has no output
    /// path relative to that directory.
    #[error("{} is not under {}", file.display(), dir.display())]
    OutsideDir { file: PathBuf, dir: PathBuf },
    #[error("{}:{error}", path.display())]
    Parse {
        path: PathBuf,
        #[source]
        error: ParseError,
    },
    /// Something the parser accepts but `.mel` code-behind does not.
    #[error("{}:{span}: {reason}", path.display())]
    Invalid {
        path: PathBuf,
        span: Span,
        reason: InvalidMarkup,
    },
}

impl BuildError {
    pub(crate) fn invalid(path: &Path, span: Span, reason: InvalidMarkup) -> Self {
        Self::Invalid {
            path: path.to_path_buf(),
            span,
            reason,
        }
    }
}
