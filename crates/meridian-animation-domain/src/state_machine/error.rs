//! Why a graph refuses to be built.

use thiserror::Error;

/// A transition that names something the graph does not have.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GraphError {
    /// A transition end is not a state of this graph.
    #[error("transition names state {0}, which the graph does not have")]
    UnknownState(usize),
    /// A condition reads a parameter this graph does not have.
    #[error("condition reads parameter {0}, which the graph does not have")]
    UnknownParam(usize),
    /// A condition reads a parameter of the wrong kind (a `Below` on a bool).
    #[error("condition reads parameter {0} as the wrong kind")]
    WrongKind(usize),
}
