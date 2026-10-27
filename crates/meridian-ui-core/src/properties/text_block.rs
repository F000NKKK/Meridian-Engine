//! `TextBlock` properties.

use crate::property::{Invalidates, Property, PropertyId, PropertyType};

pub const TEXT: Property<String> = Property::new(
    PropertyId::new(14),
    "TextBlock.Text",
    Invalidates::Layout,
    || String::new().into_value(),
);
pub const FONT_SIZE: Property<f32> = Property::new(
    PropertyId::new(15),
    "TextBlock.FontSize",
    Invalidates::Layout,
    || 14.0_f32.into_value(),
);
