//! [`LogLevel`]: how important a log line is.

use substrate_data::Schema;

/// How important a log line is. Variants are only ever appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Schema)]
pub enum LogLevel {
    /// Detail for debugging.
    Debug,
    /// Something worth knowing.
    Info,
    /// Something that may be wrong.
    Warning,
    /// Something that is wrong.
    Error,
}
