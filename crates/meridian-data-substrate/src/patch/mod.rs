//! Field-level patches of document values: how an instance's overrides are
//! merged into a prefab's component, and how a changed component is turned
//! back into one.

mod difference;
mod merge;

pub(crate) use difference::patch_between;
pub(crate) use merge::merged;
