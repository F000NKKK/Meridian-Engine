//! Bloom strength and blur settings.

/// Tunable bloom parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BloomConfig {
    /// Multiplies the blurred bright texture before the additive
    /// composite — `0.0` disables the visible effect without skipping
    /// the passes (see the adapter's `BloomPass::apply` for actually skipping them).
    pub intensity: f32,
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self { intensity: 1.8 }
    }
}
