//! Image-based lighting data: an environment as the renderer samples it.
//!
//! An [`EnvironmentMap`] is built on the CPU from an equirectangular image of
//! scene radiance, six cube faces (or a procedural [`Sky`]): a chain of GGX-prefiltered
//! levels for specular reflection and nine spherical-harmonic coefficients
//! for diffuse irradiance (ADR 035). The adapter uploads it; nothing here
//! touches a GPU.

#![expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "image sizes and texel indices are far below 2^24, and sample coordinates are clamped before they become indices"
)]

mod background;
mod cube;
mod direction;
mod error;
mod irradiance;
mod level;
mod map;
mod prefilter;
mod sky;
#[cfg(test)]
mod tests;

pub use background::Background;
pub use cube::CubeFaces;
pub use direction::{direction_to_uv, uv_to_direction};
pub use error::EnvironmentError;
pub use level::EnvironmentLevel;
pub use map::EnvironmentMap;
pub use sky::Sky;
