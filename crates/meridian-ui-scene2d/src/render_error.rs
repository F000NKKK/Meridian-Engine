//! Why a tree could not be turned into a scene.

use meridian_text_domain::FontError;
use meridian_ui_domain::UiError;
use thiserror::Error;

/// Extraction with text failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RenderError {
    /// The tree rejected a node.
    #[error("ui tree: {0}")]
    Tree(#[from] UiError),
    /// A glyph could not be rasterized.
    #[error("font: {0}")]
    Font(#[from] FontError),
}
