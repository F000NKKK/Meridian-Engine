//! `DockPanel` properties; `DOCK` is attached to the docked child.

use crate::layout::value::Dock;
use crate::property::{Invalidates, Property, PropertyId, PropertyType};

pub const DOCK: Property<Dock> = Property::new(
    PropertyId::new(8),
    "DockPanel.Dock",
    Invalidates::Layout,
    || Dock::Left.into_value(),
);
pub const LAST_CHILD_FILL: Property<bool> = Property::new(
    PropertyId::new(9),
    "DockPanel.LastChildFill",
    Invalidates::Layout,
    || true.into_value(),
);
