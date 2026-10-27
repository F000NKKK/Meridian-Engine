//! Reading a `.mres` file.

use std::path::Path;

use crate::{BuildError, ResourceSet};

/// Reads `<Set>.mres`; the set is named after the file.
pub(crate) fn read_set(source: &Path) -> Result<ResourceSet, BuildError> {
    let text = std::fs::read_to_string(source).map_err(|error| BuildError::io(source, error))?;
    let name = source
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    Ok(ResourceSet::parse(source, &name, &text)?)
}
