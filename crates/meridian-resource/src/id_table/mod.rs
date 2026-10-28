//! The table from an external name for a resource to its runtime identity.

mod bind_error;
mod binding;
mod resource_id_table;
#[cfg(test)]
mod tests;

pub use bind_error::BindError;
pub use binding::Binding;
pub use resource_id_table::ResourceIdTable;
