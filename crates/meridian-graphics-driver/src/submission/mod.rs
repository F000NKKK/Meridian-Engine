//! The submission bridge: turns a culled `Scene3d` into real GPU draws —
//! step 2 of docs/graphics-design.md's scene vocabulary implementation order
//! (texture-sampled materials), plus step 3 (Blinn-Phong lighting). It is the
//! recording half of the rendering context: the domain (`graphics-core`) owns
//! the scene, materials, meshes and handles; this module bakes them into GPU
//! buffers and records the passes.
//!
//! Mesh/material/texture identity is handle-based, never a raw index or a
//! driver object (ADR 002); four pipelines are chosen per renderable at bake
//! time: textured vs flat colour crossed with lit vs `Material::unlit`.
//!
//! Design notes: docs/notes/meridian-graphics-core.md (`submission`).

mod draw_buffers;
mod draw_kind;
mod draw_request;
mod prepared_scene;
mod registries;
mod scene_assets;
mod scene_renderer;
mod shaders;
mod shadows;
mod submit_scene3d;
#[cfg(test)]
mod tests;
mod uniform_bytes;
mod vertex_layout;

pub use draw_buffers::DrawBuffers;
pub(crate) use draw_buffers::bake_draw_buffers;
pub(crate) use draw_kind::DrawKind;
pub(crate) use draw_request::DrawRequest;
pub use prepared_scene::PreparedScene;
pub use registries::TextureRegistry;
pub use scene_assets::SceneAssets;
pub use scene_renderer::{DEFAULT_SHADOW_MAP_SIZE, SceneRenderer};
pub use shaders::*;
pub(crate) use shadows::ShadowDepthPass;
pub use submit_scene3d::submit_scene3d;
pub(crate) use uniform_bytes::{lit_uniform_bytes, mat4_to_bytes};
pub(crate) use vertex_layout::{
    colored_vertex_layout, emissive_from_colored_layout, emissive_from_textured_layout,
    shadow_from_colored_layout, shadow_from_textured_layout, textured_vertex_layout,
};
