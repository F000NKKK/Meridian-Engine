//! Typed handles for shaders.

use meridian_resource::ResourceId;

#[derive(Debug)]
pub struct ShaderMarker;

pub type ShaderHandle = ResourceId<ShaderMarker>;
