//! A stage's identity in the runtime's dependency graph.

/// Opaque handle to a stage registered with a [`Runtime`](crate::Runtime) — used to
/// declare it as another stage's dependency. Not valid across different
/// `Runtime`s, the same restriction `meridian_runtime_threads::JobId` has and
/// for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageId(pub(crate) usize);
