//! [`AssetKey`]: an asset's path under the project's asset root, and the
//! `@asset:` kind whose references carry it.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use substrate_data::schema::NamedType;
use substrate_data::{Expected, RefId, RefKey};

substrate_data::kind!(AssetPathKind = "asset": AssetKey);

/// An asset, by its path under the project's asset root (`meshes/hero.obj`):
/// any text without whitespace or control characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetKey(Box<str>);

impl AssetKey {
    /// `path` as an asset path.
    ///
    /// # Errors
    ///
    /// [`AssetKeyError`] when `path` is empty or has whitespace or control characters.
    pub fn new(path: &str) -> Result<Self, AssetKeyError> {
        if RefKey::is_valid(path) {
            Ok(AssetKey(path.into()))
        } else {
            Err(AssetKeyError {
                text: path.to_owned(),
            })
        }
    }

    /// As written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AssetKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for AssetKey {
    type Err = AssetKeyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AssetKey::new(s)
    }
}

impl NamedType for AssetKey {
    const NAME: &'static str = "asset_key";
    type Stored = String;
    fn to_stored(&self) -> String {
        self.0.to_string()
    }
    fn from_stored(stored: String) -> Option<Self> {
        AssetKey::new(&stored).ok()
    }
}

substrate_data::named_schema!(AssetKey);

impl RefId for AssetKey {
    const EXPECTED: Expected = Expected::Text;
    fn parse(spelled: &str) -> Option<Self> {
        AssetKey::new(spelled).ok()
    }
    fn spell(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }
}

/// A text that isn't a asset path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetKeyError {
    /// What was given.
    pub text: String,
}

impl fmt::Display for AssetKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` isn't a asset path (no spaces or control characters)",
            self.text
        )
    }
}

impl std::error::Error for AssetKeyError {}
