//! The handle of a job within its graph.

/// Opaque handle to a job within the [`JobGraph`] it was added to. Not
/// valid across different `JobGraph`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(pub(crate) usize);
