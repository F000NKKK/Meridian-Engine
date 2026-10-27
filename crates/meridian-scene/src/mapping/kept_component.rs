//! [`KeptComponent`]: an unknown component, kept as written.

use substrate_data::Value;

use crate::ComponentName;

/// A component the registry doesn't know, kept as written.
#[derive(Debug, Clone, PartialEq)]
pub struct KeptComponent {
    /// Its name in the document.
    pub name: ComponentName,
    /// Its value, untouched.
    pub value: Value,
}
