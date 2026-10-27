//! The component registry: component names and types, how each is read
//! into a `World` and written out of one, and what each looks like.

mod component_registry;
mod component_shape;
mod entry;
mod register_error;
mod unknown_components;

pub use component_registry::ComponentRegistry;
pub use component_shape::ComponentShape;
pub(crate) use entry::{Entry, Insert};
pub use register_error::RegisterError;
pub use unknown_components::UnknownComponents;
