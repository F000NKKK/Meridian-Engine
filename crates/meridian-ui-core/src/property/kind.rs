//! Conversion between Rust types and [`PropertyValue`].

use super::literal::LiteralError;
use super::{PropertyValue, literal};
use crate::Thickness;
use crate::layout::value::{Alignment, Dock, GridLength, Length, Orientation, Visibility};

/// A Rust type that a property can hold.
pub trait PropertyType: Sized {
    fn into_value(self) -> PropertyValue;
    /// `None` when `value` holds a different kind.
    fn from_value(value: &PropertyValue) -> Option<Self>;
    /// Parses markup text (`"Auto"`, `"4,2"`, `"2*"`) as this type.
    fn parse(text: &str) -> Result<Self, LiteralError>;
}

macro_rules! property_type {
    ($($ty:ty => $variant:ident via $parse:path),* $(,)?) => {$(
        impl PropertyType for $ty {
            fn into_value(self) -> PropertyValue {
                PropertyValue::$variant(self)
            }

            fn from_value(value: &PropertyValue) -> Option<Self> {
                match value {
                    PropertyValue::$variant(inner) => Some(inner.clone()),
                    _ => None,
                }
            }

            fn parse(text: &str) -> Result<Self, LiteralError> {
                $parse(text)
            }
        }
    )*};
}

property_type! {
    bool => Bool via literal::boolean,
    u32 => Index via literal::index,
    f32 => Float via literal::float,
    String => Text via literal::text,
    Length => Length via literal::length,
    Vec<GridLength> => GridLengths via literal::grid_lengths,
    Thickness => Thickness via literal::thickness,
    Alignment => Alignment via literal::alignment,
    Orientation => Orientation via literal::orientation,
    Dock => Dock via literal::dock,
    Visibility => Visibility via literal::visibility,
}
