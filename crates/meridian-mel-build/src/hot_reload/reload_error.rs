//! [`ReloadError`]: why a hot reload did not happen.

use std::path::PathBuf;

use meridian_ui_core::UiError;

use crate::BuildError;

/// A failed reload leaves the tree as it was; the next change of the
/// files tries again.
#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
    /// The document or a resource set does not read or check.
    #[error(transparent)]
    Build(#[from] BuildError),
    /// The tree rejected an edit.
    #[error("the UI tree rejected the reload: {0}")]
    Ui(#[from] UiError),
    /// The root of the document is not a UI element.
    #[error("{} has no UI root to reload", .0.display())]
    NotUi(PathBuf),
}
