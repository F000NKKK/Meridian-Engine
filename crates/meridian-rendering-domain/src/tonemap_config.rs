//! Exposure and tone curve of the final HDR to display step.

use crate::TonemapOperator;

/// Tunable tone mapping parameters (ADR 034).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TonemapConfig {
    /// Multiplies scene light before the curve: `1.0` leaves it as lit, above
    /// brightens, below darkens.
    pub exposure: f32,
    /// The curve that compresses the exposed light into display range.
    pub operator: TonemapOperator,
}

impl Default for TonemapConfig {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            operator: TonemapOperator::default(),
        }
    }
}
