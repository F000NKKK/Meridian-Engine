//! [`Refreshed`]: what a [`HotDocument`](super::HotDocument) did after a
//! change of its files.

use super::Rebuilt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refreshed {
    /// Same shape: `changed` attributes were set in place.
    Patched { changed: usize },
    /// The shape changed, so the document was built again; old `NodeId`s
    /// are stale and `{x:Bind}` values need applying again.
    Rebuilt(Rebuilt),
}
