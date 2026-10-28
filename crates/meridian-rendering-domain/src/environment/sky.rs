//! A procedural sky, for scenes without an environment image.

use super::direction::uv_to_direction;
use super::{EnvironmentError, EnvironmentMap};

/// Width and height of the image a [`Sky`] is rendered to before prefiltering.
const SKY_SIZE: [u32; 2] = [256, 128];

/// A sky of three colours blended by elevation, with an optional sun disc.
/// All values are linear radiance and may exceed `1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sky {
    /// Radiance straight up.
    pub zenith: [f32; 3],
    /// Radiance at the horizon.
    pub horizon: [f32; 3],
    /// Radiance straight down.
    pub ground: [f32; 3],
    /// Direction towards the sun; need not be normalised.
    pub sun_direction: [f32; 3],
    /// Radiance of the sun disc; zero for no sun.
    pub sun_radiance: [f32; 3],
    /// Angular radius of the sun disc in radians.
    pub sun_radius: f32,
}

impl Default for Sky {
    fn default() -> Self {
        Self {
            zenith: [0.25, 0.4, 0.75],
            horizon: [0.75, 0.8, 0.85],
            ground: [0.2, 0.18, 0.16],
            sun_direction: [0.4, 0.6, 0.5],
            sun_radiance: [0.0; 3],
            sun_radius: 0.05,
        }
    }
}

impl Sky {
    /// The radiance this sky has looking along unit `direction`.
    #[must_use]
    pub fn radiance(&self, direction: [f32; 3]) -> [f32; 3] {
        let elevation = direction[1].clamp(-1.0, 1.0);
        let (edge, weight) = if elevation >= 0.0 {
            (self.zenith, elevation.sqrt())
        } else {
            (self.ground, (-elevation).sqrt())
        };
        let mut color = [0, 1, 2].map(|c| self.horizon[c] + (edge[c] - self.horizon[c]) * weight);
        let sun = normalize(self.sun_direction);
        let alignment = direction[0] * sun[0] + direction[1] * sun[1] + direction[2] * sun[2];
        if alignment >= self.sun_radius.cos() {
            color = self.sun_radiance;
        }
        color
    }

    /// This sky as an equirectangular image, `width * height` texels.
    #[must_use]
    pub fn to_equirect(self, width: u32, height: u32) -> Vec<[f32; 3]> {
        let mut texels = Vec::with_capacity((width * height) as usize);
        for row in 0..height {
            for column in 0..width {
                let u = (column as f32 + 0.5) / width as f32;
                let v = (row as f32 + 0.5) / height as f32;
                texels.push(self.radiance(uv_to_direction(u, v)));
            }
        }
        texels
    }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    v.map(|c| c / length)
}

impl EnvironmentMap {
    /// Builds a map of `sky`.
    ///
    /// # Errors
    ///
    /// Never in practice: the rendered image has a valid size, so this only
    /// forwards [`EnvironmentMap::from_equirect`]'s error type.
    pub fn from_sky(sky: &Sky) -> Result<Self, EnvironmentError> {
        let [width, height] = SKY_SIZE;
        Self::from_equirect(width, height, &sky.to_equirect(width, height))
    }
}
