//! Resource identity: typed handles, versioning, lifetime and dependency tracking for textures/meshes/buffers/shaders. Not a manager — the type system resource lifecycle is built on.
//!
//! Layer: shared kernel; bounded context: identity (`craftbook.toml`,
//! docs/context-map.md).
//!
//! Deliberately does not load, cache, or evict anything (see ADR 006 /
//! docs/dependency-rules.md rule 8) — [`DependencyGraph`] tracks the
//! topology of declared dependencies between resources a caller already
//! owns elsewhere; it never owns a resource's data or decides when it's
//! freed.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod dependency_graph;
mod id_table;
mod resource_dependency;
mod resource_id;
#[cfg(test)]
mod tests;
mod version;

pub use dependency_graph::DependencyGraph;
pub use id_table::{BindError, Binding, ResourceIdTable};
pub use resource_dependency::ResourceDependency;
pub use resource_id::ResourceId;
pub use version::Version;
