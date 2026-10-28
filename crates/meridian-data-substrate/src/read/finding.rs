//! [`Finding`]: a problem this crate finds in a document.

use substrate_data::Path;

use crate::load::SceneProblem;

/// A problem found here, where it is, and whether at its key.
pub(crate) struct Finding {
    pub(crate) problem: SceneProblem,
    pub(crate) path: Path,
    pub(crate) on_key: bool,
}
