//! Properties every element has (WPF's `FrameworkElement`).

use crate::Thickness;
use crate::layout::value::{Alignment, Length, Visibility};
use crate::property::{Invalidates, Property, PropertyId, PropertyType};

pub const WIDTH: Property<Length> =
    Property::new(PropertyId::new(0), "Width", Invalidates::Layout, || {
        Length::Auto.into_value()
    });
pub const HEIGHT: Property<Length> =
    Property::new(PropertyId::new(1), "Height", Invalidates::Layout, || {
        Length::Auto.into_value()
    });
pub const MARGIN: Property<Thickness> =
    Property::new(PropertyId::new(2), "Margin", Invalidates::Layout, || {
        Thickness::ZERO.into_value()
    });
pub const HORIZONTAL_ALIGNMENT: Property<Alignment> = Property::new(
    PropertyId::new(3),
    "HorizontalAlignment",
    Invalidates::Layout,
    || Alignment::Stretch.into_value(),
);
pub const VERTICAL_ALIGNMENT: Property<Alignment> = Property::new(
    PropertyId::new(4),
    "VerticalAlignment",
    Invalidates::Layout,
    || Alignment::Stretch.into_value(),
);
pub const VISIBILITY: Property<Visibility> = Property::new(
    PropertyId::new(5),
    "Visibility",
    Invalidates::Layout,
    || Visibility::Visible.into_value(),
);
