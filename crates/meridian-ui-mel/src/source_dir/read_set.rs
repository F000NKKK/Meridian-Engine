//! Reading a `.mres` file.

use std::path::Path;

use crate::{BuildError, Culture, ResourceError, ResourceProblem, ResourceSet};

/// Reads `<Set>.mres`, or the variant `<Set>.<culture>.mres`.
pub(crate) fn read_set(source: &Path) -> Result<ResourceSet, BuildError> {
    let text = std::fs::read_to_string(source).map_err(|error| BuildError::io(source, error))?;
    let stem = source
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let Some((name, tag)) = stem.split_once('.') else {
        return Ok(ResourceSet::parse(source, &stem, &text)?);
    };
    let culture = Culture::parse(tag).ok_or_else(|| {
        ResourceError::at(
            source,
            &text,
            0,
            ResourceProblem::InvalidCulture(tag.to_string()),
        )
    })?;
    Ok(ResourceSet::parse_variant(source, name, culture, &text)?)
}
