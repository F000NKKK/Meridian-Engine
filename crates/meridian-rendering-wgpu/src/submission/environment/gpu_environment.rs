//! An environment on the GPU.

use meridian_rendering_domain::EnvironmentMap;

use super::EnvironmentUniform;
use crate::{Device, Sampler, Texture};

/// The prefiltered environment texture, its sampler and the uniform data
/// that goes with them. Without an environment a 1x1 black texture stands in
/// so the bindings are always valid, and the uniform says to ignore it.
#[derive(Debug)]
pub(crate) struct GpuEnvironment {
    pub texture: Texture,
    pub sampler: Sampler,
    pub uniform: EnvironmentUniform,
}

impl GpuEnvironment {
    /// The stand-in used while no environment is set.
    pub(crate) fn none(device: &Device) -> Self {
        Self {
            texture: device.create_neutral_environment_texture(),
            sampler: device.create_environment_sampler(),
            uniform: EnvironmentUniform::none(),
        }
    }

    /// `map` uploaded.
    pub(crate) fn of(device: &Device, map: &EnvironmentMap) -> Self {
        Self {
            texture: device.create_environment_texture(map.levels()),
            sampler: device.create_environment_sampler(),
            uniform: EnvironmentUniform::of(map),
        }
    }
}
