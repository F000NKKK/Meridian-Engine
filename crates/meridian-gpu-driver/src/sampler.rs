//! A texture sampler.

/// A texture sampler — filtering/addressing configuration for reading a
/// [`Texture`](crate::Texture) in a shader. Built by [`Device::create_sampler`](crate::Device::create_sampler).
#[derive(Debug)]
pub struct Sampler {
    pub(crate) raw: wgpu::Sampler,
}
