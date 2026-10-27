//! Archetype-based, data-oriented Entity Component System with SoA storage, built for cache- and SIMD-friendly iteration.
//!
//! Layer: shared kernel; bounded context: entity (`craftbook.toml`,
//! docs/context-map.md). A performance kernel (Craftbook §7.8): its
//! data-oriented layout is provisional until a benchmark justifies it
//! (ADR 019 D8).
//!
//! `World` owns everything: entity allocation, archetypes, and per-
//! archetype Structure-of-Arrays component storage. Entities move between
//! archetypes on [`World::insert`]/[`World::remove`] — the SoA columns are
//! type-erased (via `std::any::Any` + downcasting) so `World` never needs
//! to know every `Component` type that will ever exist, but every move is
//! still a safe, checked downcast, no `unsafe`.
//!
//! Scope, deliberately: [`Query`]/[`QueryMut`] iterate a *single*
//! component type across every archetype that has it. Multi-component
//! queries (`Query<(&Transform, &mut Velocity)>`) need to prove to the
//! borrow checker that two different columns don't alias, which is a
//! genuinely harder, `unsafe`-adjacent problem — deferred until there's a
//! system that actually needs it, not built speculatively. See
//! docs/ecs-design.md.

#![forbid(unsafe_code)]

mod archetype;
mod column;
mod component;
mod entity;
mod entity_location;
mod query;
mod query_mut;
#[cfg(test)]
mod tests;
mod transform;
mod world;

use archetype::Archetype;
use column::{AnyColumn, Column};
pub use component::Component;
pub use entity::Entity;
use entity_location::EntityLocation;
pub use query::Query;
pub use query_mut::QueryMut;
pub use transform::Transform;
pub use world::World;
