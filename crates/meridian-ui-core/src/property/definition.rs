//! A property's static definition and its typed handle.

use core::marker::PhantomData;

use super::{LiteralError, PropertyId, PropertyType, PropertyValue};

/// What changing a property invalidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalidates {
    /// Only presentation; layout results stay valid.
    Nothing,
    /// Measure and arrange must run again.
    Layout,
}

/// Type-erased facts about a property, for tooling and the registry.
#[derive(Debug, Clone, Copy)]
pub struct PropertyMeta {
    pub id: PropertyId,
    /// Qualified name as written in markup: `Width`, `Grid.Row`.
    pub name: &'static str,
    pub invalidates: Invalidates,
    pub default: fn() -> PropertyValue,
    /// Parses markup text as this property's type.
    pub parse: fn(&str) -> Result<PropertyValue, LiteralError>,
}

/// A typed handle to a registered property. Reading and writing through it
/// is checked at compile time; no property is addressed by string at run
/// time.
#[derive(Debug)]
pub struct Property<T> {
    meta: PropertyMeta,
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Property<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Property<T> {}

impl<T: PropertyType> Property<T> {
    pub const fn new(
        id: PropertyId,
        name: &'static str,
        invalidates: Invalidates,
        default: fn() -> PropertyValue,
    ) -> Self {
        Self {
            meta: PropertyMeta {
                id,
                name,
                invalidates,
                default,
                parse: parse_value::<T>,
            },
            marker: PhantomData,
        }
    }

    pub const fn meta(self) -> PropertyMeta {
        self.meta
    }
}

fn parse_value<T: PropertyType>(text: &str) -> Result<PropertyValue, LiteralError> {
    T::parse(text).map(PropertyType::into_value)
}
