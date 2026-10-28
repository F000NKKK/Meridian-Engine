//! The prefiltered environment the renderer samples.

use super::irradiance::{evaluate, irradiance_coefficients};
use super::prefilter::prefilter;
use super::{EnvironmentError, EnvironmentLevel};

/// How many specular levels a map holds: level `i` is prefiltered for
/// roughness `i / (SPECULAR_LEVELS - 1)` and is `width >> i` by `height >> i`
/// (at least one texel), the size of mip `i` of a texture of the first level.
pub(super) const SPECULAR_LEVELS: usize = 6;

/// Widest image kept as the sharpest specular level; larger sources are
/// averaged down to it.
const MAX_WIDTH: u32 = 256;

/// Width at which the halved images used as prefilter sources stop.
const MIN_SOURCE_WIDTH: u32 = 8;

/// An environment ready for image-based lighting: six
/// prefiltered images (sharpest first) and the diffuse irradiance as
/// spherical-harmonic coefficients.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentMap {
    levels: Vec<EnvironmentLevel>,
    irradiance: [[f32; 3]; 9],
}

impl EnvironmentMap {
    /// Builds a map from an equirectangular image of linear radiance:
    /// `texels` is `width * height` values, row-major from the top row, `y` up
    /// (see [`uv_to_direction`](super::uv_to_direction) for the orientation).
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::Empty`] for a zero dimension,
    /// [`EnvironmentError::TexelCountMismatch`] when `texels` is the wrong
    /// length.
    pub fn from_equirect(
        width: u32,
        height: u32,
        texels: &[[f32; 3]],
    ) -> Result<Self, EnvironmentError> {
        if width == 0 || height == 0 {
            return Err(EnvironmentError::Empty);
        }
        let expected = width as usize * height as usize;
        if texels.len() != expected {
            return Err(EnvironmentError::TexelCountMismatch {
                expected,
                actual: texels.len(),
            });
        }
        let mut sharpest = EnvironmentLevel {
            width,
            height,
            texels: texels.to_vec(),
        };
        while sharpest.width > MAX_WIDTH {
            sharpest = sharpest.halved();
        }
        let mut pyramid = vec![sharpest];
        while pyramid[pyramid.len() - 1].width > MIN_SOURCE_WIDTH {
            pyramid.push(pyramid[pyramid.len() - 1].halved());
        }
        Ok(Self::from_pyramid(&pyramid))
    }

    /// Prefilters every level from the `pyramid` of progressively halved
    /// images, whose first entry is the sharpest.
    fn from_pyramid(pyramid: &[EnvironmentLevel]) -> Self {
        let sharpest = &pyramid[0];
        let mut levels = vec![sharpest.clone()];
        for index in 1..SPECULAR_LEVELS {
            let roughness = index as f32 / (SPECULAR_LEVELS - 1) as f32;
            let width = (sharpest.width >> index).max(1);
            let height = (sharpest.height >> index).max(1);
            levels.push(prefilter(pyramid, roughness, width, height));
        }
        let coarse = pyramid
            .iter()
            .find(|level| level.width <= 64)
            .unwrap_or(&pyramid[pyramid.len() - 1]);
        Self {
            levels,
            irradiance: irradiance_coefficients(coarse),
        }
    }

    /// The prefiltered specular images, sharpest first.
    #[must_use]
    pub fn levels(&self) -> &[EnvironmentLevel] {
        &self.levels
    }

    /// The nine spherical-harmonic coefficients of irradiance divided by
    /// `pi`, ready for the shader's evaluation of
    /// [`irradiance_at`](Self::irradiance_at).
    #[must_use]
    pub fn irradiance(&self) -> &[[f32; 3]; 9] {
        &self.irradiance
    }

    /// What a diffuse surface of albedo 1 facing unit `normal` reflects of
    /// this environment.
    #[must_use]
    pub fn irradiance_at(&self, normal: [f32; 3]) -> [f32; 3] {
        evaluate(&self.irradiance, normal)
    }
}
