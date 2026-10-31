//! Schema adapter for the independent local translation component.

use meridian_ecs::Position;
use meridian_gac::Vec3;
use substrate_data::with::With;
use substrate_data::{Reader, Schema, Shape, Value, ValueRef};

/// Stores Position as the same vec3 form used throughout the spatial SDK.
#[derive(Debug, Clone, Copy, Default)]
pub struct PositionCodec;

impl With<Position> for PositionCodec {
    fn shape() -> Shape {
        Vec3::shape()
    }
    fn read(value: &ValueRef<'_>, reader: &mut Reader<'_>) -> Option<Position> {
        Vec3::read(value, reader).map(Position)
    }
    fn write(position: &Position) -> Value {
        position.0.write()
    }
}
