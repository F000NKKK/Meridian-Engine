//! [`PrefabPath`]: a prefab's path under the project's asset root, and the
//! `@prefab:` kind whose references carry it.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use substrate_data::schema::NamedType;
use substrate_data::{Expected, RefId, RefKey};

substrate_data::kind!(PrefabKind = "prefab": PrefabPath);

/// A prefab, by its path under the project's asset root (`actors/hero`):
/// any text without whitespace or control characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PrefabPath(Box<str>);

impl PrefabPath {
    /// `path` as a prefab path.
    ///
    /// # Errors
    ///
    /// [`PrefabPathError`] when `path` is empty or has whitespace or control characters.
    pub fn new(path: &str) -> Result<Self, PrefabPathError> {
        if RefKey::is_valid(path) {
            Ok(PrefabPath(path.into()))
        } else {
            Err(PrefabPathError {
                text: path.to_owned(),
            })
        }
    }

    /// As written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The one of `known` closest to `self`, when close enough to be a typo.
    pub(crate) fn closest<'k>(
        &self,
        known: impl Iterator<Item = &'k PrefabPath>,
    ) -> Option<&'k PrefabPath> {
        super::closest_spelling(&self.0, known, |k| k.as_str())
    }
}

impl fmt::Display for PrefabPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for PrefabPath {
    type Err = PrefabPathError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PrefabPath::new(s)
    }
}

impl NamedType for PrefabPath {
    const NAME: &'static str = "prefab_path";
    type Stored = String;
    fn to_stored(&self) -> String {
        self.0.to_string()
    }
    fn from_stored(stored: String) -> Option<Self> {
        PrefabPath::new(&stored).ok()
    }
}

substrate_data::named_schema!(PrefabPath);

impl RefId for PrefabPath {
    const EXPECTED: Expected = Expected::Text;
    fn parse(spelled: &str) -> Option<Self> {
        PrefabPath::new(spelled).ok()
    }
    fn spell(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }
}

/// A text that isn't a prefab path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabPathError {
    /// What was given.
    pub text: String,
}

impl fmt::Display for PrefabPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` isn't a prefab path (no spaces or control characters)",
            self.text
        )
    }
}

impl std::error::Error for PrefabPathError {}
