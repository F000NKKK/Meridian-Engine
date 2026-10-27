//! `Grid` properties; `ROW`/`COLUMN` are attached to the child.

use crate::layout::value::GridLength;
use crate::property::{Invalidates, Property, PropertyId, PropertyType};

pub const ROW: Property<u32> =
    Property::new(PropertyId::new(10), "Grid.Row", Invalidates::Layout, || {
        0_u32.into_value()
    });
pub const COLUMN: Property<u32> = Property::new(
    PropertyId::new(11),
    "Grid.Column",
    Invalidates::Layout,
    || 0_u32.into_value(),
);
pub const ROW_DEFINITIONS: Property<Vec<GridLength>> = Property::new(
    PropertyId::new(12),
    "Grid.RowDefinitions",
    Invalidates::Layout,
    || Vec::<GridLength>::new().into_value(),
);
pub const COLUMN_DEFINITIONS: Property<Vec<GridLength>> = Property::new(
    PropertyId::new(13),
    "Grid.ColumnDefinitions",
    Invalidates::Layout,
    || Vec::<GridLength>::new().into_value(),
);
