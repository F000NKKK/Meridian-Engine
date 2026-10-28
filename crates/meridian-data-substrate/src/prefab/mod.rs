//! Prefabs: where their bytes come from ([`PrefabSource`], a port the
//! application implements) and how instancing expands them.

mod expansion;
mod instance;
mod library;
mod source;
mod unavailable;

pub use expansion::MAX_PREFAB_DEPTH;
pub use instance::PrefabInstance;
pub use library::PrefabLibrary;
pub use source::PrefabSource;
pub use unavailable::PrefabUnavailable;
