//! Archetype-based, data-oriented Entity Component System with SoA storage, built for cache- and SIMD-friendly iteration.

use core::marker::PhantomData;
use meridian_gac_core::Motor3;

/// An opaque, generational entity id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}

/// Marker trait for plain-data component types. No behavior — see
/// docs/ecs-design.md.
pub trait Component: 'static {}

/// The set of component types an entity has; entities sharing an archetype
/// share Structure-of-Arrays storage.
#[derive(Debug, Default)]
pub struct Archetype {
    pub component_type_ids: Vec<core::any::TypeId>,
}

/// Structure-of-Arrays storage for one archetype.
#[derive(Debug, Default)]
pub struct Storage;

/// Iterates the component columns of every archetype matching `T`.
#[derive(Debug, Default)]
pub struct Query<T> {
    _marker: PhantomData<T>,
}

/// The engine-wide spatial component: rotation + translation as a single
/// motor, shared with physics/graphics/audio (see docs/gac-design.md).
#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub motor: Motor3,
}

impl Component for Transform {}
