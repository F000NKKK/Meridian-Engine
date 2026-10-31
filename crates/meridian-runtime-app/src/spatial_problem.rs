//! Invalid spatial source combinations.

use meridian_gac::RotationError;

/// Invalid or competing local pose sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialProblem {
    /// Motor3 cannot coexist with Position or Rotor as another pose authority.
    CompetingSources,
    /// A position or motor coefficient was not finite.
    NonFinite,
    /// The independent rotor could not be normalized.
    Rotation(RotationError),
}
