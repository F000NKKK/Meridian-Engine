//! [`HotDocument`]: a `.meui` file kept in step with its tree.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use meridian_ui_domain::{NodeId, UiTree};

use super::{LiveUi, Refreshed, Reload, ReloadError, SourceWatcher};
use crate::source_dir::{SourceFiles, read_sets};
use crate::{BuildError, Culture, MeuiDocument, ResourceSets, UiNode};

/// A document and the resource sets it uses, watched on disk. The host
/// calls [`Self::refresh`] each frame (or at any rhythm it likes); when
/// the `.meui` or a `.mres` changed, the document is read again and the
/// tree follows, patched in place when the shape is the same and built
/// again when it is not.
#[derive(Debug)]
pub struct HotDocument {
    meui: PathBuf,
    resources: Option<PathBuf>,
    watcher: SourceWatcher,
    live: LiveUi,
    culture: Option<Culture>,
    /// The culture was changed since the last read.
    switched: bool,
}

impl HotDocument {
    /// Watches `meui` and, when given, every `.mres` under `resources`.
    /// `root` is the document's element in the tree, built from the files
    /// as they are now (the compiled `build_ui` from the same files).
    pub fn open(
        meui: &Path,
        resources: Option<&Path>,
        root: NodeId,
        debounce: Duration,
    ) -> Result<Self, ReloadError> {
        let (ui, watched) = Self::read(meui, resources, None)?;
        Ok(Self {
            meui: meui.to_path_buf(),
            resources: resources.map(Path::to_path_buf),
            watcher: SourceWatcher::new(watched, debounce),
            live: LiveUi::new(root, ui),
            culture: None,
            switched: false,
        })
    }

    /// Reads `@res:` references in `culture` from now on (`None`: the
    /// default sets). Takes effect at the next [`Self::refresh`], which
    /// applies it even when no file changed: this is the run-time
    /// language switch.
    pub fn set_culture(&mut self, culture: Option<Culture>) {
        if self.culture != culture {
            self.culture = culture;
            self.switched = true;
        }
    }

    /// Applies a change of the files, if one has settled. `None` when
    /// nothing changed. A file that does not read or check is an error and
    /// leaves the tree as it was; the next change tries again.
    pub fn refresh(
        &mut self,
        tree: &mut UiTree,
        now: Instant,
    ) -> Result<Option<Refreshed>, ReloadError> {
        let changed = !self.watcher.poll(now).is_empty();
        if !changed && !self.switched {
            return Ok(None);
        }
        self.switched = false;
        let (ui, _) = Self::read(&self.meui, self.resources.as_deref(), self.culture.as_ref())?;
        let refreshed = match self.live.reload(tree, ui.clone())? {
            Reload::Patched { changed } => Refreshed::Patched { changed },
            Reload::Restructured => Refreshed::Rebuilt(self.live.rebuild(tree, ui)?),
        };
        Ok(Some(refreshed))
    }

    /// The document and the files it depends on.
    fn read(
        meui: &Path,
        resources: Option<&Path>,
        culture: Option<&Culture>,
    ) -> Result<(UiNode, Vec<PathBuf>), ReloadError> {
        let mut watched = vec![meui.to_path_buf()];
        let mut sets = ResourceSets::default();
        if let Some(dir) = resources {
            let mres = SourceFiles::find(dir)?.mres;
            sets = read_sets(&mres)?;
            if let Some(culture) = culture {
                sets = sets.for_culture(culture);
            }
            watched.extend(mres);
        }
        let text = std::fs::read_to_string(meui).map_err(|error| BuildError::io(meui, error))?;
        let ui = MeuiDocument::parse_with(meui, &text, &sets)?
            .ui
            .ok_or_else(|| ReloadError::NotUi(meui.to_path_buf()))?;
        Ok((ui, watched))
    }
}
