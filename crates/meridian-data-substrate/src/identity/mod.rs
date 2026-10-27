//! The typed names a scene document uses: component names, entity keys,
//! prefab paths and display names — each parsed once where it is read.

mod component_name;
mod display_name;
mod entity_key;
mod prefab_path;

pub use component_name::{ComponentName, ComponentNameError};
pub use display_name::DisplayName;
pub use entity_key::{EntityKey, EntityKeyError, EntityKind};
pub use prefab_path::{PrefabKind, PrefabPath, PrefabPathError};
