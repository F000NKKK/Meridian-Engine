//! [`SourceWatcher`]: which source files changed, without blocking.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::file_stamp::FileStamp;

/// Polls the modification stamps of a fixed list of files. It never
/// blocks and never spawns a thread: the host calls [`Self::poll`] from
/// its frame loop, as often as it likes. A change is reported once the
/// file has been quiet for the debounce, so an editor's burst of writes
/// is one change.
///
/// Reading a stamp is a bounded local call, so this stays synchronous
/// (CLAUDE.md, async only on genuine I/O); an OS-notification source can
/// replace the polling later behind the same `poll`.
#[derive(Debug)]
pub struct SourceWatcher {
    files: Vec<Watched>,
    debounce: Duration,
}

#[derive(Debug)]
struct Watched {
    path: PathBuf,
    seen: Option<FileStamp>,
    /// When the stamp last moved, until the change is reported.
    moved_at: Option<Instant>,
}

impl SourceWatcher {
    /// Watches `paths`, taking their present stamps as the baseline.
    pub fn new(paths: impl IntoIterator<Item = PathBuf>, debounce: Duration) -> Self {
        let files = paths
            .into_iter()
            .map(|path| Watched {
                seen: FileStamp::of(&path),
                path,
                moved_at: None,
            })
            .collect();
        Self { files, debounce }
    }

    /// The files that changed and have been quiet for the debounce, each
    /// reported once per change.
    pub fn poll(&mut self, now: Instant) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        for file in &mut self.files {
            let stamp = FileStamp::of(&file.path);
            if stamp != file.seen {
                file.seen = stamp;
                file.moved_at = Some(now);
            }
            let quiet = file
                .moved_at
                .is_some_and(|at| now.saturating_duration_since(at) >= self.debounce);
            if quiet {
                file.moved_at = None;
                changed.push(file.path.clone());
            }
        }
        changed
    }
}
