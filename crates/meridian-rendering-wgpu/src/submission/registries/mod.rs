//! GPU-resident registries.

mod neutral_textures;
mod texture_registry;

pub(crate) use neutral_textures::NeutralTextures;
pub use texture_registry::TextureRegistry;
