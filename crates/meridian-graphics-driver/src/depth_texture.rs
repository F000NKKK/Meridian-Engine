//! A depth attachment texture.

/// A depth/stencil-comparable texture — see [`Device::create_depth_texture`](crate::Device::create_depth_texture).
#[derive(Debug)]
pub struct DepthTexture {
    pub(crate) texture: meridian_gpu_driver::Texture,
}
