//! The syntax tree a `.meui` document parses into: elements, attributes,
//! names and attribute values.

mod attribute;
mod bracket_depth;
mod element;
mod markup_extension;
mod markup_value;
mod prefix;
mod qualified_name;

pub use attribute::Attribute;
pub use element::Element;
pub use markup_extension::MarkupExtension;
pub use markup_value::{MarkupValue, MarkupValueError, MarkupValueErrorKind};
pub use prefix::Prefix;
pub(crate) use qualified_name::is_identifier_byte;
pub use qualified_name::{QualifiedName, QualifiedNameError};
