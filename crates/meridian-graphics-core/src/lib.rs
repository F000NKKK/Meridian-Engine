//! High-level rendering: render graph, scene extraction, visibility/culling, lighting, materials, camera and post-processing.

use meridian_gac_core::Projection;
use meridian_resource_core::ResourceId;

/// Marker types distinguishing `ResourceId`s of different graphics resource
/// kinds — see docs/adr/006-resource-core-separation.md.
pub struct TextureMarker;
pub struct MeshMarker;
pub struct ShaderMarker;

pub type TextureHandle = ResourceId<TextureMarker>;
pub type MeshHandle = ResourceId<MeshMarker>;
pub type ShaderHandle = ResourceId<ShaderMarker>;

/// A node in the render graph: declares its resource reads/writes; the
/// graph derives execution order from that, not from manual sequencing.
#[derive(Debug, Clone, Default)]
pub struct RenderPass {
    pub name: &'static str,
}

/// An automatically-ordered set of render passes for one frame.
#[derive(Debug, Clone, Default)]
pub struct RenderGraph {
    pub passes: Vec<RenderPass>,
}

/// A camera's view + projection.
#[derive(Debug, Clone, Copy, Default)]
pub struct Camera {
    pub projection: Projection,
}

/// A surface's shading inputs.
#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub albedo: TextureHandle,
}
