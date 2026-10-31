//! Typed handles for shaders.

use meridian_resource::ResourceId;

/// Marker type distinguishing shader resource identifiers.
#[derive(Debug)]
pub struct ShaderMarker;

/// Generational resource identifier for a shader.
pub type ShaderHandle = ResourceId<ShaderMarker>;
