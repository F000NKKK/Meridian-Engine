//! Independently authored local translation.

use meridian_gac::Vec3;

use crate::Component;

/// Translation relative to the parent; combines with an optional Rotor component.
/// A spatial resolution system produces Transform for consumers of rigid poses.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Position(pub Vec3);

impl Component for Position {}
