//! [`MetaError`]: why a `.meta` file did not read.

use substrate_data::Diagnostic;
use thiserror::Error;

use super::META_FORMAT;
use crate::AssetId;

/// Why a `.meta` file did not read. Nothing is returned for a file with a
/// problem: it is read whole or not at all.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum MetaError {
    /// Text that is not UTF-8.
    #[error("the file is not UTF-8 text (valid up to byte {valid_up_to})")]
    NotUtf8 {
        /// How many leading bytes are valid.
        valid_up_to: usize,
    },
    /// Bytes that start as the binary form (`SDAT`) but do not decode, at
    /// the byte that shows it.
    #[error("byte {offset}: {message}")]
    Malformed {
        /// What is wrong.
        message: String,
        /// The byte it was found at.
        offset: usize,
    },
    /// The document is not a map of the `.meta` fields.
    #[error("not a .meta file: the root is not a map")]
    NotAMap,
    /// No `format` field, or one that is not a whole number from 1 up.
    #[error("the file has no `format`, or it is not a whole number from 1 up")]
    BadFormat,
    /// A newer `format` than this reads: the file is from a newer version.
    #[error("unsupported .meta format {found} (this reads up to {META_FORMAT})")]
    UnsupportedFormat {
        /// The format written.
        found: u32,
    },
    /// Syntax errors and fields that are missing, unknown or of the wrong
    /// type, each with its path (and line and column in text).
    #[error("{}", joined(.0))]
    Invalid(Vec<Diagnostic>),
    /// `settings` is present but is not a map.
    #[error("`settings` is not a map")]
    SettingsNotAMap,
    /// An asset that depends on itself.
    #[error("asset {id} lists itself in `dependencies`")]
    SelfDependency {
        /// The asset.
        id: AssetId,
    },
    /// The same dependency listed twice.
    #[error("`dependencies` lists {id} twice (entries {first} and {second})")]
    DuplicateDependency {
        /// The dependency.
        id: AssetId,
        /// Its first position.
        first: usize,
        /// Its second position.
        second: usize,
    },
}

fn joined(problems: &[Diagnostic]) -> String {
    let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
    lines.join("; ")
}
