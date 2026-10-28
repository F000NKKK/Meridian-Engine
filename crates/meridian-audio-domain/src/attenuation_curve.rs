//! How gain falls between the reference and the maximum distance.

/// The shape of the distance rolloff, after OpenAL's distance models. Every
/// curve is `1.0` at or inside the reference distance and is clamped at the
/// maximum distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttenuationCurve {
    /// `ref / (ref + rolloff * (d - ref))`: the physical default, never
    /// reaching zero.
    #[default]
    Inverse,
    /// `1 - rolloff * (d - ref) / (max - ref)`: falls straight to silence at
    /// the maximum distance (with a rolloff of 1), then stays there.
    Linear,
    /// `(d / ref) ^ -rolloff`: a steeper, power-law fall.
    Exponential,
}
