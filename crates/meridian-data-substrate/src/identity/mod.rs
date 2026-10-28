//! The typed names a scene document uses: component names, entity keys,
//! prefab paths and display names — each parsed once where it is read.

mod asset_key;
mod component_name;
mod display_name;
mod edit_distance;
mod entity_key;
mod prefab_path;

pub use asset_key::{AssetKey, AssetKeyError, AssetPathKind};
pub use component_name::{ComponentName, ComponentNameError};
pub use display_name::DisplayName;
pub(crate) use edit_distance::closest_spelling;
pub use entity_key::{EntityKey, EntityKeyError, EntityKind};
pub use prefab_path::{PrefabKind, PrefabPath, PrefabPathError};
