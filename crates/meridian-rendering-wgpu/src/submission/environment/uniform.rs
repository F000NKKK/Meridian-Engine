//! The environment's share of the lit uniform buffer.

use meridian_rendering_domain::EnvironmentMap;

/// What the lit shaders read about the environment from their uniform buffer:
/// whether there is one, how many mip levels its texture has and the nine
/// irradiance coefficients.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EnvironmentUniform {
    enabled: bool,
    max_lod: f32,
    irradiance: [[f32; 3]; 9],
}

impl EnvironmentUniform {
    /// Bytes this occupies in the uniform: nine padded `vec3`s and one `vec4`.
    pub(crate) const BYTES: usize = 9 * 16 + 16;

    /// No environment: the shaders fall back to the scene's ambient colours.
    pub(crate) fn none() -> Self {
        Self {
            enabled: false,
            max_lod: 0.0,
            irradiance: [[0.0; 3]; 9],
        }
    }

    /// The uniform describing `map`.
    pub(crate) fn of(map: &EnvironmentMap) -> Self {
        Self {
            enabled: true,
            max_lod: u8::try_from(map.levels().len().saturating_sub(1)).map_or(0.0, f32::from),
            irradiance: *map.irradiance(),
        }
    }

    /// Whether an environment is set.
    pub(crate) fn enabled(self) -> bool {
        self.enabled
    }

    /// The highest mip level of the environment texture.
    pub(crate) fn max_lod(self) -> f32 {
        self.max_lod
    }

    /// The `env_sh` array then the `env_params` vector, as the shader lays them out.
    pub(crate) fn to_bytes(self) -> [u8; Self::BYTES] {
        let mut bytes = [0u8; Self::BYTES];
        let mut put = |offset: usize, value: f32| {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        };
        for (index, coefficient) in self.irradiance.iter().enumerate() {
            for (channel, value) in coefficient.iter().enumerate() {
                put(index * 16 + channel * 4, *value);
            }
        }
        put(9 * 16, f32::from(u8::from(self.enabled)));
        put(9 * 16 + 4, self.max_lod);
        bytes
    }
}
