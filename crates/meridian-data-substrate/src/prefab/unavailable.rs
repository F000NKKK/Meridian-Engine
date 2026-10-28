//! [`PrefabUnavailable`]: why a [`super::PrefabSource`] has no bytes.

use std::fmt;

use crate::PrefabPath;

/// Why a prefab's bytes could not be given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefabUnavailable {
    /// Why, for a person (`no such prefab`, the I/O error's text, …).
    pub reason: String,
    /// The path of a prefab there is that this is probably a typo of.
    pub suggestion: Option<PrefabPath>,
}

impl PrefabUnavailable {
    /// No prefab is there, and none is close.
    #[must_use]
    pub fn missing() -> Self {
        PrefabUnavailable {
            reason: "no such prefab".to_owned(),
            suggestion: None,
        }
    }
}

impl fmt::Display for PrefabUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.reason)?;
        match &self.suggestion {
            Some(s) => write!(f, " — did you mean `{s}`?"),
            None => Ok(()),
        }
    }
}

impl std::error::Error for PrefabUnavailable {}
