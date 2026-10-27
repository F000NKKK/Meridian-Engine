//! Type-erased component columns of an archetype.

use crate::Component;
use std::any::Any;

/// A type-erased `SoA` column, downcast back to `Column<T>` by every
/// operation that needs the concrete type. Object-safe by hand (`Any`
/// isn't dyn-compatible for downcasting on its own) — every method here
/// is what a caller holding only `&dyn AnyColumn` can still do safely.
/// `Send` follows from `Component: Send` — see that trait's own doc
/// comment for why.
pub(crate) trait AnyColumn: Send {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// A fresh, empty column of the same concrete type — how a new
    /// archetype clones the *shape* of an existing one without `World`
    /// ever needing to know the concrete `T`.
    fn new_same_type(&self) -> Box<dyn AnyColumn>;
    /// Removes `row` from this column and pushes it onto `dest` (which
    /// must be the same concrete type — an archetype move never changes a
    /// surviving column's element type). Panics on a type mismatch; that
    /// would be an internal `World` bug, not a caller error.
    fn move_row_to(&mut self, row: usize, dest: &mut dyn AnyColumn);
    /// Drops `row` without moving it anywhere (the component isn't in the
    /// destination archetype's signature).
    fn swap_remove_erased(&mut self, row: usize);
}

pub(crate) struct Column<T>(pub(crate) Vec<T>);

impl<T: Component> AnyColumn for Column<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn new_same_type(&self) -> Box<dyn AnyColumn> {
        Box::new(Column::<T>(Vec::new()))
    }
    fn move_row_to(&mut self, row: usize, dest: &mut dyn AnyColumn) {
        let value = self.0.swap_remove(row);
        let dest = dest
            .as_any_mut()
            .downcast_mut::<Column<T>>()
            .expect("AnyColumn::move_row_to: destination column type mismatch");
        dest.0.push(value);
    }
    fn swap_remove_erased(&mut self, row: usize) {
        self.0.swap_remove(row);
    }
}
