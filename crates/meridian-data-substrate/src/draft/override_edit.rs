//! [`OverrideEdit`]: what an override does to a component.

use substrate_data::Value;

/// What an override does to the component.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OverrideEdit {
    /// The component is this, whole (`components`).
    Replace(Value),
    /// This is merged into the component's document value (`fields`).
    Merge(Value),
    /// The component is taken away (`remove`).
    Remove,
}
