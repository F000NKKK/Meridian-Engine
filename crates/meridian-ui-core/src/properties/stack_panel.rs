//! `StackPanel` properties.

use crate::layout::value::Orientation;
use crate::property::{Invalidates, Property, PropertyId, PropertyType};

pub const ORIENTATION: Property<Orientation> = Property::new(
    PropertyId::new(7),
    "StackPanel.Orientation",
    Invalidates::Layout,
    || Orientation::Vertical.into_value(),
);
