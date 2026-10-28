//! Prefabs: where their bytes come from ([`PrefabSource`], a port the
//! application implements) and how instancing expands them.

mod prefab_expansion;
mod prefab_instance;
mod prefab_library;
mod prefab_source;
mod prefab_unavailable;

pub use prefab_expansion::MAX_PREFAB_DEPTH;
pub use prefab_instance::PrefabInstance;
pub use prefab_library::PrefabLibrary;
pub use prefab_source::PrefabSource;
pub use prefab_unavailable::PrefabUnavailable;
