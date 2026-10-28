//! [`DocumentKind`]: which document a writer writes.

/// Which document a writer writes: they differ in the `name` a scene has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocumentKind {
    /// `.mscene`.
    Scene,
    /// `.mprefab`.
    Prefab,
}
