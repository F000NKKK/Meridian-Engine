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
//!   here. `ecs-core` gets no `data` feature (team/core's decision, and
//!   [ADR 016](../../../docs/adr/016-data-formats-on-substrate-data.md)'s
//!   decision 3); instead [`TransformData`], a newtype of this crate,
//!   implements `Schema` and converts to and from `Transform`, and
//!   [`TransformCodec`] registers `Transform` through it
//!   ([`ComponentRegistry::register_with`] — the same door for any
//!   component whose crate can't implement `Schema`).
//! - **Registered by name and by type.** One name per component type and
//!   one type per name; [`ComponentRegistry::name_of`] finds a type's name.
//!   Saving walks the registered types for each entity (until `ecs-core`
//!   can list an entity's component types).
//! - **Runtime handles never reach a file.** `Entity { index, generation }`
//!   is process-local; documents use their own string ids, remapped to
//!   freshly spawned entities on load ([`SceneMap`]).
//! - **Loading is all-or-nothing.** Every problem in a document is
//!   collected (with line, column and path) before the `World` is touched;
//!   a document with any error spawns nothing.
//! - **Hierarchy as a [`Parent`] component, here for now.** `ecs-core` has
//!   no parent concept yet; `parent: "@entity:…"` is checked against the
//!   document's ids and inserted as this crate's [`Parent`] component
//!   (also kept in [`SceneMap`]). It moves to `ecs-core` once that has a
//!   hierarchy (docs/roadmap.md).
//! - **Thin over `substrate-data`.** Its API is used in three places only
//!   — `TransformData`, the registry's read/write function pointers, and
//!   `document.rs` — so its coming typed-API change stays a local edit.
//! - **Prefabs are referenced, not instanced yet.** `prefab: "@prefab:…"`
//!   goes through the caller's resolver and is kept in the map; `.mprefab`
//!   instancing is designed (docs/data-formats.md), not implemented.

mod document;
mod parent;
mod registry;
mod transform;

pub use document::{FORMAT, SceneEntity, SceneMap};
pub use parent::Parent;
pub use registry::ComponentRegistry;
pub use transform::{TransformCodec, TransformData};

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
