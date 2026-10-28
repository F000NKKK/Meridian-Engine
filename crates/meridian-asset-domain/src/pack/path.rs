//! [`PackPath`]: the name of an asset inside a pack.

use std::fmt;
use std::str::FromStr;

use substrate_data::schema::NamedType;

/// The longest path, in bytes.
const MAX_LEN: usize = 1024;

/// An asset's path inside a pack (`meshes/crate.obj`): `/`-separated
/// segments, relative to the pack's root, none empty, `.` or `..`, with no
/// `\`, control characters or leading or trailing `/`. The same shape an
/// asset source takes, so a pack can stand in for a directory.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackPath(Box<str>);

impl PackPath {
    /// `path` as a pack path.
    ///
    /// # Errors
    ///
    /// [`PackPathError`] when `path` is not of the shape above.
    pub fn new(path: &str) -> Result<Self, PackPathError> {
        let plain = |c: char| !c.is_control() && c != '\\';
        let segments_right = path
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | "..") && segment.chars().all(plain));
        if path.len() <= MAX_LEN && segments_right {
            Ok(PackPath(path.into()))
        } else {
            Err(PackPathError {
                text: path.chars().take(64).collect(),
            })
        }
    }

    /// As written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PackPath {
    type Err = PackPathError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PackPath::new(s)
    }
}

impl NamedType for PackPath {
    const NAME: &'static str = "pack_path";
    type Stored = String;
    fn to_stored(&self) -> String {
        self.0.to_string()
    }
    fn from_stored(stored: String) -> Option<Self> {
        PackPath::new(&stored).ok()
    }
}

substrate_data::named_schema!(PackPath);

/// A text that is not a pack path (the start of it, at most 64 characters).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "`{text}` is not a pack path (`/`-separated, relative, no empty, `.` or `..` segment, no `\\`)"
)]
pub struct PackPathError {
    /// What was given.
    pub text: String,
}
