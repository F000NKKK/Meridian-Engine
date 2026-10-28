//! `ScrollViewer` properties.

use crate::property::{Invalidates, Property, PropertyId, PropertyType};

/// How far the content is scrolled to the left, in pixels. Out-of-range
/// values are clamped when the viewer is arranged.
pub const HORIZONTAL_OFFSET: Property<f32> = Property::new(
    PropertyId::new(20),
    "ScrollViewer.HorizontalOffset",
    Invalidates::Layout,
    || 0.0_f32.into_value(),
);

/// How far the content is scrolled up, in pixels. Out-of-range values are
/// clamped when the viewer is arranged.
pub const VERTICAL_OFFSET: Property<f32> = Property::new(
    PropertyId::new(21),
    "ScrollViewer.VerticalOffset",
    Invalidates::Layout,
    || 0.0_f32.into_value(),
);
