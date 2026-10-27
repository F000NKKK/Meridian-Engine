//! [`ResourceEntry`]: one named resource of a set.

use super::ResourceValue;

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceEntry {
    /// As written: `title`, `screen.title`.
    pub name: String,
    /// The generated constant: `TITLE`, `SCREEN_TITLE`.
    pub constant: String,
    pub value: ResourceValue,
    /// A note for other developers; shown in the generated docs.
    pub comment: Option<String>,
}
