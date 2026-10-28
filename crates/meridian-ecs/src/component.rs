//! The marker trait for data attachable to an entity.

/// Marker trait for plain-data component types. No behavior — see
/// docs/ecs-design.md. `Send` (not just `'static`) so `World` itself is
/// `Send` — required for `runtime-app::Runtime` to hold a `World` behind
/// a lock shared across its `JobGraph`-dispatched stages (see that
/// crate's own module doc); every real component in this workspace
/// (`Transform`, `MeshRenderer`, ...) is plain data and already `Send`
/// automatically, so this costs nothing in practice.
pub trait Component: 'static + Send {}
