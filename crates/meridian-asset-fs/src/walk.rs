//! Walking a directory for the asset paths under it.

use std::io;
use std::path::Path;

/// Pushes the `/`-separated path (below `prefix`) of every regular file
/// under `dir` onto `found`. Symbolic links and names that are not UTF-8
/// are left out.
pub(crate) fn walk(dir: &Path, prefix: &str, found: &mut Vec<String>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let mut relative = String::from(prefix);
        if !relative.is_empty() {
            relative.push('/');
        }
        relative.push_str(&name);
        let kind = entry.file_type()?;
        if kind.is_dir() {
            walk(&entry.path(), &relative, found)?;
        } else if kind.is_file() {
            found.push(relative);
        }
    }
    Ok(())
}
