//! [`Layer`]: one place assets are read from.

use std::path::PathBuf;

/// One place the application's assets live. A closed set, so a layout is
/// chosen by variant, never by guessing from a file name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layer {
    /// Loose files under this directory (development, mods).
    Directory(PathBuf),
    /// A cooked `.mpak` file (shipping, patches).
    Pack(PathBuf),
}
