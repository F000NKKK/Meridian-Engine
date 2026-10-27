//! [`RegisterError`]: why a component can't be registered.

use std::fmt;

use crate::ComponentName;

/// Why a component can't be registered: one name per type, one type per name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterError {
    /// The name is already another type's.
    NameTaken {
        /// The name.
        name: ComponentName,
    },
    /// The type is already registered, under another name.
    TypeTaken {
        /// The type, for a person.
        type_name: &'static str,
        /// Its name.
        registered_as: ComponentName,
    },
}

impl fmt::Display for RegisterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegisterError::NameTaken { name } => {
                write!(f, "component `{name}` is already registered")
            }
            RegisterError::TypeTaken {
                type_name,
                registered_as,
            } => {
                write!(
                    f,
                    "`{type_name}` is already registered as `{registered_as}`"
                )
            }
        }
    }
}

impl std::error::Error for RegisterError {}
