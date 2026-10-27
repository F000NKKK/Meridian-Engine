//! Adding a `.mres` set to a collection.

use std::path::Path;

use crate::{BuildError, ResourceError, ResourceProblem, ResourceSet, ResourceSets};

/// Adds `set`, read from `source`; two files naming one set is an error.
pub(crate) fn insert_set(
    sets: &mut ResourceSets,
    set: ResourceSet,
    source: &Path,
) -> Result<(), BuildError> {
    let name = set.name.clone();
    if sets.insert(set) {
        return Ok(());
    }
    Err(ResourceError {
        path: source.to_path_buf(),
        line: 1,
        column: 1,
        problem: ResourceProblem::DuplicateSet(name),
    }
    .into())
}
