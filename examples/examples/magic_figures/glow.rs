//! A shape's emissive color.

use meridian_sdk::dsl::dsl_tag;

/// This example's own glow color tag — not part of `meridian_sdk::dsl`
/// (the SDK has no concept of "emissive tint"), registered alongside
/// the SDK's built-ins in [`load_scene`](crate::load_scene).
#[dsl_tag(name = "Glow")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Glow {
    pub(crate) r: f32,
    pub(crate) g: f32,
    pub(crate) b: f32,
}
