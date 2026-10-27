//! [`DisplayName`]: a name shown to people.

use std::fmt;

use substrate_data::schema::NamedType;

/// A name for people (`"Level 1"`, `"Hero"`): free text, never used to
/// find anything.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayName(String);

impl DisplayName {
    /// `text` as a display name.
    pub fn new(text: impl Into<String>) -> Self {
        DisplayName(text.into())
    }

    /// As written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DisplayName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl NamedType for DisplayName {
    const NAME: &'static str = "display_name";
    type Stored = String;
    fn to_stored(&self) -> String {
        self.0.clone()
    }
    fn from_stored(stored: String) -> Option<Self> {
        Some(DisplayName(stored))
    }
}

substrate_data::named_schema!(DisplayName);
