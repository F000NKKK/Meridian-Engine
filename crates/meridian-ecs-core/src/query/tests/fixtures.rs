//! Components and spawners shared by the query tests.

use crate::{Component, Entity, World};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Position(pub(super) f32);
impl Component for Position {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Velocity(pub(super) f32);
impl Component for Velocity {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Mass(pub(super) f32);
impl Component for Mass {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Tag;
impl Component for Tag {}

pub(super) fn spawn_moving(world: &mut World, x: f32, v: f32) -> Entity {
    let e = world.spawn();
    world.insert(e, Position(x));
    world.insert(e, Velocity(v));
    e
}
