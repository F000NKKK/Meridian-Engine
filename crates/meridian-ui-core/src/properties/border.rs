//! `Border` properties.

use crate::Thickness;
use crate::property::{Invalidates, Property, PropertyId, PropertyType};

pub const PADDING: Property<Thickness> = Property::new(
    PropertyId::new(6),
    "Border.Padding",
    Invalidates::Layout,
    || Thickness::ZERO.into_value(),
);
