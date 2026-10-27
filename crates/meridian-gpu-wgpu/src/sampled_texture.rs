//! A texture paired with the sampler that reads it.

use crate::{Sampler, Texture};

/// A texture together with the sampler a shader reads it through.
#[derive(Debug, Clone, Copy)]
pub struct SampledTexture<'a> {
    /// The texture.
    pub texture: &'a Texture,
    /// The sampler used to read it.
    pub sampler: &'a Sampler,
}
