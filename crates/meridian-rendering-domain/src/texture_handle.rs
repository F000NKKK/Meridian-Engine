//! Typed handles for textures.

use meridian_resource::ResourceId;

/// Marker types distinguishing `ResourceId`s of different graphics resource
/// kinds — see docs/adr/006-resource-core-separation.md.
#[derive(Debug)]
pub struct TextureMarker;

pub type TextureHandle = ResourceId<TextureMarker>;
