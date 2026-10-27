//! The stored representation of a property value.

use crate::Thickness;
use crate::layout::value::{Alignment, Dock, GridLength, Length, Orientation, Visibility};

/// What a node stores for a property. Typed access goes through
/// [`crate::PropertyType`]; this enum is also the tooling-facing form
/// (inspector, designer) where the type is only known at run time.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    Bool(bool),
    Index(u32),
    Float(f32),
    Text(String),
    Length(Length),
    GridLengths(Vec<GridLength>),
    Thickness(Thickness),
    Alignment(Alignment),
    Orientation(Orientation),
    Dock(Dock),
    Visibility(Visibility),
}
