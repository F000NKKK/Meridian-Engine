//! The curve that maps scene light to display range.

/// How [`TonemapConfig`](crate::TonemapConfig) compresses linear scene light
/// (any positive value) into `0.0..=1.0` for the display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TonemapOperator {
    /// Clamps to `0.0..=1.0`: no compression, so a lit surface brighter than
    /// 1.0 clips to white. The behaviour before HDR rendering.
    Clamp,
    /// `x / (1 + x)`: keeps every value distinguishable, but greys out the
    /// midtones.
    Reinhard,
    /// Narkowicz's fit of the ACES filmic curve: contrast in the midtones, a
    /// soft shoulder for highlights.
    #[default]
    Aces,
}

impl TonemapOperator {
    /// The value the tone mapping shader switches on (a float, because it
    /// travels in a uniform of floats).
    #[must_use]
    pub const fn shader_index(self) -> f32 {
        match self {
            Self::Clamp => 0.0,
            Self::Reinhard => 1.0,
            Self::Aces => 2.0,
        }
    }
}
