//! High-level rendering domain: render graph, scene extraction, visibility/culling, lighting, materials, camera and sprite batching.
//!
//! Layer: domain; bounded context: rendering (`craftbook.toml`,
//! docs/context-map.md).

#![forbid(unsafe_code)]

mod bloom_config;
mod camera;
mod fly;
mod frustum;
mod graph_resource_id;
mod mesh_handle;
mod render_graph;
mod render_graph_error;
mod render_pass;
pub mod scene;
mod shader_handle;
mod shapes;
pub mod sprites;
pub mod submission;
#[cfg(test)]
mod tests;
mod texture_handle;

pub use bloom_config::BloomConfig;
pub use camera::{Camera, look_at_rotor};
pub use fly::{FlyCamera, FlyControls};
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
pub use shapes::{
    cube_mesh_source, ground_mesh_source, icosphere_mesh_source, pyramid_mesh_source,
    smooth_normals,
};
pub use sprites::{
    ClipRect, ClipSpan, ScissorRect, SpriteBatch, SpriteVertex, SpriteView, batch_scene2d,
    batch_scene2d_clipped,
};
pub use submission::{
    MaterialRegistry, MeshRegistry, MeshRegistryError, MeshSource, SHADOW_VOLUME_HALF_EXTENT,
    ShadowLight, ViewUniforms, directional_shadow_camera,
};
pub use texture_handle::{TextureHandle, TextureMarker};
