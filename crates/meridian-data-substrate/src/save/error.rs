//! [`SaveError`]: why a save didn't read.

use substrate_data::Diagnostic;
use thiserror::Error;

use crate::LoadError;

/// Why a save didn't read. Nothing is spawned when a load fails.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum SaveError {
    /// Bytes that are not the binary form (`SDAT`) or are damaged, at the
    /// byte that shows it.
    #[error("byte {offset}: {message}")]
    Malformed {
        /// What's wrong.
        message: String,
        /// The byte it was found at.
        offset: usize,
    },
    /// The binary form, but not a save's envelope: the root is not a map.
    #[error("not a save: the root is not a map of the envelope's fields")]
    NotAnEnvelope,
    /// An envelope field that isn't there.
    #[error("the save has no `{field}`")]
    MissingField {
        /// The field.
        field: &'static str,
    },
    /// An envelope field of another type than it takes.
    #[error("the save's `{field}` is not {expected}")]
    BadField {
        /// The field.
        field: &'static str,
        /// What it takes.
        expected: &'static str,
    },
    /// A `format` this crate doesn't read: the save is from a newer
    /// version (or not a save).
    #[error("unsupported save format {found} (this reads {supported})")]
    UnsupportedFormat {
        /// The format written.
        found: u32,
        /// The format this reads.
        supported: u32,
    },
    /// The scene in the save doesn't load; paths in it are within the
    /// scene document (`entities[3].components.transform`).
    #[error("the save's scene: {0}")]
    Scene(LoadError),
    /// The game's state doesn't read as the game's schema, each problem
    /// with its path under `state`. The binary named form is strict: a
    /// field the game's type doesn't know (a newer game's) is one of them.
    #[error("the save's state: {}", list(.0))]
    State(Vec<Diagnostic>),
}

fn list(problems: &[Diagnostic]) -> String {
    let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
    lines.join("; ")
}
