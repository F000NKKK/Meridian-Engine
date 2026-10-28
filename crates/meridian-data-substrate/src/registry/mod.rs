//! The component registry: component names and types, how each is read
//! into a `World` and written out of one, and what each looks like.

mod component_access;
mod component_edit_error;
mod component_registry;
mod component_shape;
mod entry;
mod game_saving;
mod instance_saving;
mod prefab_loading;
mod read_component;
mod register_error;
mod scene_loading;
mod unknown_components;

pub use component_edit_error::ComponentEditError;
pub use component_registry::ComponentRegistry;
pub use component_shape::ComponentShape;
pub(crate) use entry::Entry;
pub(crate) use read_component::Insert;
pub use register_error::RegisterError;
pub use unknown_components::UnknownComponents;
