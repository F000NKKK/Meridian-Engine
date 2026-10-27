//! Reading every `.mres` of a list into one collection.

use std::path::PathBuf;

use super::{insert_set, read_set};
use crate::{BuildError, ResourceSets};

/// The sets of `paths`; two files naming one set is an error.
pub(crate) fn read_sets(paths: &[PathBuf]) -> Result<ResourceSets, BuildError> {
    let mut sets = ResourceSets::default();
    for path in paths {
        insert_set(&mut sets, read_set(path)?, path)?;
    }
    Ok(sets)
}
