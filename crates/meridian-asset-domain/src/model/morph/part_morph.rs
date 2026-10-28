//! How one part blends between shapes.

use super::MorphTarget;

/// The blend shapes of a part and the weights it starts with. The shown
/// shape is the base vertices plus each target's offsets scaled by its weight.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PartMorph {
    /// The targets; each has one offset per vertex.
    pub targets: Vec<MorphTarget>,
    /// The weight of each target before any animation, one per target.
    pub weights: Vec<f32>,
}
