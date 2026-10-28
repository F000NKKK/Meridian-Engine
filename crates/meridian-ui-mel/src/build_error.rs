//! [`BuildError`]: why a `.meui` document could not be compiled.

use std::path::{Path, PathBuf};

use meridian_markup::{ParseError, Span};

use crate::{InvalidMarkup, ResourceError};

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
    #[error(transparent)]
    Resource(#[from] ResourceError),
    /// Something the parser accepts but `.meui` code-behind does not.
    #[error("{}:{span}: {reason}", path.display())]
    Invalid {
        path: PathBuf,
        span: Span,
        reason: InvalidMarkup,
    },
}

impl BuildError {
    pub(crate) fn io(path: &Path, error: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            error,
        }
    }

    pub(crate) fn invalid(path: &Path, span: Span, reason: InvalidMarkup) -> Self {
        Self::Invalid {
            path: path.to_path_buf(),
            span,
            reason,
        }
    }
}
