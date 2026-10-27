//! [`DuplicateKey`]: a key already in a scene.

use std::fmt;

use crate::EntityKey;

/// A key already in the scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateKey {
    /// The key.
    pub key: EntityKey,
}

impl fmt::Display for DuplicateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "entity `{}` is already in the scene", self.key)
    }
}

impl std::error::Error for DuplicateKey {}
