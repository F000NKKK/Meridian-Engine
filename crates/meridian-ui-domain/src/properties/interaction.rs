//! Properties that decide how an element takes part in input.

use crate::property::{Invalidates, Property, PropertyId, PropertyType};

/// Whether the element can hold keyboard focus.
pub const FOCUSABLE: Property<bool> = Property::new(
    PropertyId::new(16),
    "Focusable",
    Invalidates::Nothing,
    || false.into_value(),
);

/// Whether the element itself is a pointer target. `false` makes it
/// transparent to the pointer; its children still are targets.
pub const IS_HIT_TEST_VISIBLE: Property<bool> = Property::new(
    PropertyId::new(17),
    "IsHitTestVisible",
    Invalidates::Nothing,
    || true.into_value(),
);
