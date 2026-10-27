//! The typed half of the DSL story: [`DslTag`] is what
//! `meridian-markup-macros`' `#[dsl_tag(name = "...")]` implements for a
//! game developer's own struct, and [`DslRegistry`] is what
//! `meridian-sdk::dsl` builds a real scene tree from. Still
//! domain-blind: this module knows the *shape* of "a typed thing
//! parseable from attributes", not which types exist — `RigidBody`,
//! `Mesh`, and any custom tag a game registers are all just entries in
//! the same `HashMap`, added the same way.

mod built_node;
mod dsl_registry;
mod dsl_tag;
mod parse_error;

pub use built_node::BuiltNode;
pub use dsl_registry::DslRegistry;
pub use dsl_tag::{DslTag, text_attr};
pub use parse_error::TagParseError;
