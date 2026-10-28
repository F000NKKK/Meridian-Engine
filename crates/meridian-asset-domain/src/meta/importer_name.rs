//! [`ImporterName`]: which importer reads an asset's source file.

use std::fmt;
use std::str::FromStr;

use substrate_data::schema::NamedType;

/// The longest importer name.
const MAX_LEN: usize = 64;

/// The name of the importer that turns an asset's source file into
/// content (`obj`, `png`, `gltf`): lower case letters, digits, `_` and
/// `-`, starting with a letter. Which names exist is the application's
/// registry; the file only says which one it wants.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImporterName(Box<str>);

impl ImporterName {
    /// `name` as an importer name.
    ///
    /// # Errors
    ///
    /// [`ImporterNameError`] when `name` is empty, too long, does not start
    /// with a lower case letter, or has other characters than lower case
    /// letters, digits, `_` and `-`.
    pub fn new(name: &str) -> Result<Self, ImporterNameError> {
        let mut chars = name.chars();
        let starts_right = chars.next().is_some_and(|c| c.is_ascii_lowercase());
        let rest_right =
            chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-".contains(c));
        if starts_right && rest_right && name.len() <= MAX_LEN {
            Ok(ImporterName(name.into()))
        } else {
            Err(ImporterNameError {
                text: name.to_owned(),
            })
        }
    }

    /// As written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ImporterName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ImporterName {
    type Err = ImporterNameError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ImporterName::new(s)
    }
}

impl NamedType for ImporterName {
    const NAME: &'static str = "importer_name";
    type Stored = String;
    fn to_stored(&self) -> String {
        self.0.to_string()
    }
    fn from_stored(stored: String) -> Option<Self> {
        ImporterName::new(&stored).ok()
    }
}

substrate_data::named_schema!(ImporterName);

/// A text that is not an importer name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "`{text}` is not an importer name (lower case letters, digits, `_` and `-`, starting with a letter, at most 64 bytes)"
)]
pub struct ImporterNameError {
    /// What was given.
    pub text: String,
}
