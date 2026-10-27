//! [`Reload`]: what a hot reload did.

/// The result of [`LiveUi::reload`](super::LiveUi::reload).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reload {
    /// The document has the same elements; `changed` attributes were set
    /// again (or reset to their default) in place. Every `NodeId`,
    /// including the `Names`, is still valid.
    Patched { changed: usize },
    /// An element, `x:Name` or child was added, removed or retyped. The
    /// tree is untouched: the host builds the document again with the
    /// compiled `build_ui`, or restarts.
    Restructured,
}
