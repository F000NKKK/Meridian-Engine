//! High-level rendering: render graph, scene extraction, visibility/culling, lighting, materials, camera and post-processing.
//!
//! Layer: domain; bounded context: rendering (`craftbook.toml`,
//! docs/context-map.md).

#![forbid(unsafe_code)]

pub mod bloom;
mod camera;
mod frustum;
mod graph_resource_id;
mod mesh_handle;
mod render_graph;
mod render_graph_error;
mod render_pass;
pub mod scene;
mod shader_handle;
pub mod submission;
#[cfg(test)]
mod tests;
mod texture_handle;

pub use bloom::{BloomConfig, BloomInput, BloomPass};
pub use camera::{Camera, look_at_rotor};
pub use frustum::Frustum;
pub use graph_resource_id::GraphResourceId;
pub use mesh_handle::{MeshHandle, MeshMarker};
pub use render_graph::RenderGraph;
pub use render_graph_error::RenderGraphError;
pub use render_pass::RenderPass;
pub use scene::{
    BlendMode, Camera2d, FrameScene, Light, Material, MaterialHandle, Mesh, MeshRenderer,
    Renderable3d, Scene2d, Scene3d, Sprite, View, cull_scene3d, extract_scene3d,
    world_space_bounds,
};
pub use shader_handle::{ShaderHandle, ShaderMarker};
pub use submission::{
    DEFAULT_SHADOW_MAP_SIZE, DrawBuffers, MAX_LIGHTS, MaterialRegistry, MeshRegistry,
    MeshRegistryError, MeshSource, PreparedScene, SHADOW_VOLUME_HALF_EXTENT, SceneAssets,
    SceneRenderer, ShadowLight, TEXTURED_SHADER_WGSL, TextureRegistry, UNLIT_SHADER_WGSL,
    ViewUniforms, directional_shadow_camera, submit_scene3d,
};
pub use texture_handle::{TextureHandle, TextureMarker};
