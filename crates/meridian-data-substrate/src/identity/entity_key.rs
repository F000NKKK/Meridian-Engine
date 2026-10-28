//! [`EntityKey`]: an entity's stable id within one document, and the
//! `@entity:` kind whose references carry it.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use substrate_data::schema::NamedType;
use substrate_data::{Expected, RefId, RefKey};

substrate_data::kind!(EntityKind = "entity": EntityKey);

/// An entity's id within its document (`hero`, `door_left`): stable across
/// saves, unlike the process-local `Entity` handle it maps to on load. Any
/// text without whitespace or control characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityKey(Box<str>);

impl EntityKey {
    /// `key` as an entity key.
    ///
    /// # Errors
    ///
    /// [`EntityKeyError`] when `key` is empty or has whitespace or control characters.
    pub fn new(key: &str) -> Result<Self, EntityKeyError> {
        if RefKey::is_valid(key) {
            Ok(EntityKey(key.into()))
        } else {
            Err(EntityKeyError {
                text: key.to_owned(),
            })
        }
    }

    /// As written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `local` under `self`, the way a prefab instance names the entities
    /// of its prefab: `hero/gun` (`local` `gun` under `hero`).
    #[must_use]
    pub fn under(&self, local: &EntityKey) -> EntityKey {
        EntityKey([&*self.0, "/", &*local.0].concat().into())
    }
}

impl fmt::Display for EntityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for EntityKey {
    type Err = EntityKeyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        EntityKey::new(s)
    }
}

impl NamedType for EntityKey {
    const NAME: &'static str = "entity_key";
    type Stored = String;
    fn to_stored(&self) -> String {
        self.0.to_string()
    }
    fn from_stored(stored: String) -> Option<Self> {
        EntityKey::new(&stored).ok()
    }
}

substrate_data::named_schema!(EntityKey);

impl RefId for EntityKey {
    const EXPECTED: Expected = Expected::Text;
    fn parse(spelled: &str) -> Option<Self> {
        EntityKey::new(spelled).ok()
    }
    fn spell(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }
}

/// A text that isn't an entity key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityKeyError {
    /// What was given.
    pub text: String,
}

impl fmt::Display for EntityKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` isn't an entity key (no spaces or control characters)",
            self.text
        )
    }
}

impl std::error::Error for EntityKeyError {}
