//! Scene serialization on [Substrate Data](https://github.com/F000NKKK/Substrate-Data):
//! the `.mscene` document schema, an application-built
//! [`ComponentRegistry`], and `World` load/save. See docs/data-formats.md
//! and [ADR 016](../../../docs/adr/016-data-formats-on-substrate-data.md).
//!
//! Tier: above `meridian-ecs-core` (and `meridian-gac-core`, whose `data`
//! feature gives `Motor3` its schema), below `meridian-engine-core`. It
//! knows no component of any subsystem: an application registers the
//! components its scenes may hold, by name.
//!
//! ```
//! use meridian_ecs_core::{Transform, World};
//! use meridian_scene::{ComponentRegistry, NoResolver, TransformCodec};
//!
//! let mut registry = ComponentRegistry::new();
//! registry.register_with::<Transform, TransformCodec>("transform");
//!
//! let text = r#"{
//!   format: 1,
//!   entities: [
//!     { id: "hero", components: { transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 1.5, 0] } } } },
//!   ],
//! }"#;
//! let mut world = World::new();
//! let scene = registry.load_scene(text, &mut world, &mut NoResolver).unwrap();
//! let hero = scene.entity("hero").unwrap();
//! assert!(world.get::<Transform>(hero).is_some());
//! let saved = registry.save_scene(&world, &scene);
//! assert!(saved.contains("\"hero\""));
//! ```
//!
//! # Decisions
//!
//! - **No globals.** The registry is a plain value the application builds
//!   and passes around ([ADR 003](../../../docs/adr/003-no-global-managers.md)).
//! - **`ecs-core` stays file-blind.** `Transform` lives in `ecs-core`, and
//!   the orphan rule forbids implementing `substrate_data::Schema` for it
//!   here. Rather than give `ecs-core` a `data` feature (which would make
//!   the ECS crate know about a file format, against
//!   [ADR 016](../../../docs/adr/016-data-formats-on-substrate-data.md)'s
//!   decision 3), a component can be registered through a codec
//!   (`substrate_data::with::With`): [`TransformCodec`] is the one for
//!   `Transform`, and any application type whose crate can't implement
//!   `Schema` uses the same door ([`ComponentRegistry::register_with`]).
//! - **Loading is all-or-nothing.** Every problem in a document is
//!   collected (with line, column and path) before the `World` is touched;
//!   a document with any error spawns nothing.
//! - **Hierarchy is recorded, not applied.** `ecs-core` has no parent
//!   concept yet, so `parent: "@entity:…"` is checked against the
//!   document's ids and kept in the [`SceneMap`] ([`SceneEntity::parent`],
//!   [`SceneMap::parent_of`]); turning it into an ECS relationship is a
//!   later step (docs/roadmap.md).
//! - **Prefabs are referenced, not instanced yet.** `prefab: "@prefab:…"`
//!   goes through the caller's resolver and is kept in the map; `.mprefab`
//!   instancing is designed (docs/data-formats.md), not implemented.

mod document;
mod registry;
mod transform;

pub use document::{FORMAT, SceneEntity, SceneMap};
pub use registry::ComponentRegistry;
pub use transform::TransformCodec;

pub use substrate_data::{Diagnostic, Errors, Resolve};

substrate_data::kind!(
    EntityKind = "entity",
    PrefabKind = "prefab",
    AssetKind = "asset"
);

/// A [`Resolve`] that accepts every reference as written — for scenes
/// whose asset/prefab references the caller checks later (or not at all).
/// `@entity:` references are still checked against the document.
pub struct NoResolver;

impl Resolve for NoResolver {}
