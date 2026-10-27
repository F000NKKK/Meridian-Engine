//! Typed views of an archetype's type-erased columns.

use crate::{AnyColumn, Column, Component};

/// The `C` column as a slice, or `None` if `column` holds another type.
pub(crate) fn typed<C: Component>(column: &dyn AnyColumn) -> Option<&[C]> {
    column
        .as_any()
        .downcast_ref::<Column<C>>()
        .map(|column| column.0.as_slice())
}

/// The `C` column as a mutable slice, or `None` if `column` holds another type.
pub(crate) fn typed_mut<C: Component>(column: &mut dyn AnyColumn) -> Option<&mut [C]> {
    column
        .as_any_mut()
        .downcast_mut::<Column<C>>()
        .map(|column| column.0.as_mut_slice())
}
