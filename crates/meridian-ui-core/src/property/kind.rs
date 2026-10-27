//! Conversion between Rust types and [`PropertyValue`].

use super::PropertyValue;
use crate::Thickness;
use crate::layout::value::{Alignment, Dock, GridLength, Length, Orientation, Visibility};

/// A Rust type that a property can hold.
pub trait PropertyType: Sized {
    fn into_value(self) -> PropertyValue;
    /// `None` when `value` holds a different kind.
    fn from_value(value: &PropertyValue) -> Option<Self>;
}

macro_rules! property_type {
    ($($ty:ty => $variant:ident),* $(,)?) => {$(
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
        }
    )*};
}

property_type! {
    bool => Bool,
    u32 => Index,
    f32 => Float,
    String => Text,
    Length => Length,
    Vec<GridLength> => GridLengths,
    Thickness => Thickness,
    Alignment => Alignment,
    Orientation => Orientation,
    Dock => Dock,
    Visibility => Visibility,
}
