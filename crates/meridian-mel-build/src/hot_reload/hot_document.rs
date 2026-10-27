//! [`HotDocument`]: a `.mel` file kept in step with its tree.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use meridian_ui_core::{NodeId, UiTree};

use super::{LiveUi, Refreshed, Reload, ReloadError, SourceWatcher};
use crate::source_dir::{SourceFiles, read_sets};
use crate::{BuildError, MelDocument, ResourceSets, UiNode};

/// A document and the resource sets it uses, watched on disk. The host
/// calls [`Self::refresh`] each frame (or at any rhythm it likes); when
/// the `.mel` or a `.mres` changed, the document is read again and the
/// tree follows, patched in place when the shape is the same and built
/// again when it is not.
#[derive(Debug)]
pub struct HotDocument {
    mel: PathBuf,
    resources: Option<PathBuf>,
    watcher: SourceWatcher,
    live: LiveUi,
}

impl HotDocument {
    /// Watches `mel` and, when given, every `.mres` under `resources`.
    /// `root` is the document's element in the tree, built from the files
    /// as they are now (the compiled `build_ui` from the same files).
    pub fn open(
        mel: &Path,
        resources: Option<&Path>,
        root: NodeId,
        debounce: Duration,
    ) -> Result<Self, ReloadError> {
        let (ui, watched) = Self::read(mel, resources)?;
        Ok(Self {
            mel: mel.to_path_buf(),
            resources: resources.map(Path::to_path_buf),
            watcher: SourceWatcher::new(watched, debounce),
            live: LiveUi::new(root, ui),
        })
    }

    /// Applies a change of the files, if one has settled. `None` when
    /// nothing changed. A file that does not read or check is an error and
    /// leaves the tree as it was; the next change tries again.
    pub fn refresh(
        &mut self,
        tree: &mut UiTree,
        now: Instant,
    ) -> Result<Option<Refreshed>, ReloadError> {
        if self.watcher.poll(now).is_empty() {
            return Ok(None);
        }
        let (ui, _) = Self::read(&self.mel, self.resources.as_deref())?;
        let refreshed = match self.live.reload(tree, ui.clone())? {
            Reload::Patched { changed } => Refreshed::Patched { changed },
            Reload::Restructured => Refreshed::Rebuilt(self.live.rebuild(tree, ui)?),
        };
        Ok(Some(refreshed))
    }

    /// The document and the files it depends on.
    fn read(mel: &Path, resources: Option<&Path>) -> Result<(UiNode, Vec<PathBuf>), ReloadError> {
        let mut watched = vec![mel.to_path_buf()];
        let mut sets = ResourceSets::default();
        if let Some(dir) = resources {
            let mres = SourceFiles::find(dir)?.mres;
            sets = read_sets(&mres)?;
            watched.extend(mres);
        }
        let text = std::fs::read_to_string(mel).map_err(|error| BuildError::io(mel, error))?;
        let ui = MelDocument::parse_with(mel, &text, &sets)?
            .ui
            .ok_or_else(|| ReloadError::NotUi(mel.to_path_buf()))?;
        Ok((ui, watched))
    }
}
