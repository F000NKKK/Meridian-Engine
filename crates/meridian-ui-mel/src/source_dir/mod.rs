//! Finding and reading the `.mel` / `.mres` files under a directory, for
//! the build script and for hot reload.

mod insert_set;
mod read_set;
mod read_sets;
mod source_files;

pub(crate) use insert_set::insert_set;
pub(crate) use read_set::read_set;
pub(crate) use read_sets::read_sets;
pub(crate) use source_files::SourceFiles;
