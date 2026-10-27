//! Typed dependency properties: ids, values and typed handles.

mod definition;
mod id;
mod kind;
mod literal;
mod value;

pub use definition::{Invalidates, Property, PropertyMeta};
pub use id::PropertyId;
pub use kind::PropertyType;
pub use literal::LiteralError;
pub use value::PropertyValue;
