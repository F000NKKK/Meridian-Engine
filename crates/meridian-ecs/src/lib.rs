//! Archetype-based, data-oriented Entity Component System with `SoA` storage, built for cache- and SIMD-friendly iteration.
//!
//! Layer: shared kernel; bounded context: entity (`craftbook.toml`,
//! docs/context-map.md). A performance kernel (Craftbook §7.8): its
//! data-oriented layout is provisional until a benchmark justifies it
//! (ADR 020 D8).
//!
//! `World` owns everything: entity allocation, archetypes, and per-
//! archetype Structure-of-Arrays component storage. Entities move between
//! archetypes on [`World::insert`]/[`World::remove`] — the `SoA` columns are
//! type-erased (via `std::any::Any` + downcasting) so `World` never needs
//! to know every `Component` type that will ever exist, but every move is
//! still a safe, checked downcast, no `unsafe`.
//!
//! [`Query`]/[`QueryMut`] iterate a *single* component type across every
//! archetype that has it. [`World::query_many`]/[`World::query_many_mut`]
//! iterate two to four at once (`(&Position, &mut Velocity)`). The borrow
//! checker is satisfied without `unsafe`: an archetype's columns live in a
//! `HashMap`, and one pass over it hands each accessed column to exactly
//! one tuple element. See docs/ecs-design.md.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod archetype;
mod column;
mod commands;
mod component;
mod entity;
mod entity_location;
mod query;
mod snapshot;
#[cfg(test)]
mod tests;
mod tick;
mod transform;
mod world;

use archetype::Archetype;
use column::{AnyColumn, Column};
pub use commands::Commands;
pub use component::Component;
pub use entity::Entity;
use entity_location::EntityLocation;
pub use query::{
    Changed, Fetch, OptionalRows, Query, QueryMany, QueryManyMut, QueryMut, ReadFetch,
    ReadSelection, Selection, With, Without,
};
pub use snapshot::{Cloners, SnapshotError, WorldSnapshot};
pub use tick::Tick;
pub use transform::Transform;
pub use world::World;
