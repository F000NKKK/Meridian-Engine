//! `.mres` resource sets (ADR 017): named texts, numbers, flags and
//! colors in a Substrate-Data text document, with generated typed
//! constants and `@res:Set.name` references in markup.

mod generated_resources;
mod local_resources;
mod read_entry;
mod read_markup;
mod resource_entry;
mod resource_error;
mod resource_problem;
mod resource_set;
mod resource_sets;
mod resource_tag;
mod resource_value;

pub use generated_resources::GeneratedResources;
pub use local_resources::LocalResources;
pub(crate) use read_markup::read_markup_value;
pub use resource_entry::ResourceEntry;
pub use resource_error::ResourceError;
pub use resource_problem::ResourceProblem;
pub use resource_set::ResourceSet;
pub use resource_sets::ResourceSets;
pub(crate) use resource_tag::ResourceTag;
pub use resource_value::ResourceValue;
