//! The submission bridge: turns a culled `Scene3d` into real GPU draws —
//! step 2 of docs/graphics-design.md's scene vocabulary implementation order
//! (texture-sampled materials), plus step 3 (metallic-roughness lighting). It is the
//! recording half of the rendering context: the domain (`rendering-domain`) owns
//! the scene, materials, meshes and handles; this module bakes them into GPU
//! buffers and records the passes.
//!
//! Mesh/material/texture identity is handle-based, never a raw index or a
//! driver object (ADR 002); sixteen pipelines are chosen per renderable at bake
//! time: textured vs flat colour, lit vs `Material::unlit`, and the four
//! `DrawState`s (opaque, translucent, and the two on-top states).
//!
//! Design notes: docs/notes/meridian-rendering-domain.md (`submission`).

mod bake_batches;
mod environment;
mod gpu_mesh;
mod instance_data;
mod main_pipelines;
mod mesh_cache;
mod prepared_batch;
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

pub(crate) use bake_batches::bake_batches;
pub(crate) use environment::GpuEnvironment;
pub(crate) use prepared_batch::DrawKind;
pub use prepared_batch::PreparedBatch;
pub use prepared_scene::PreparedScene;
pub use registries::TextureRegistry;
pub use scene_assets::SceneAssets;
pub use scene_renderer::{DEFAULT_SHADOW_MAP_SIZE, SceneRenderer};
pub use shaders::*;
pub(crate) use shadows::{CascadeCasters, ShadowDepthPass};
pub use submit_scene3d::submit_scene3d;
pub(crate) use uniform_bytes::{LIT_UNIFORM_BYTES, lit_uniform_bytes, mat4_to_bytes};
pub(crate) use vertex_layout::{
    emissive_instance_layout, emissive_mapped_instance_layout, instance_vertex_layout,
    mesh_vertex_layout, position_only_mesh_layout, shadow_instance_layout,
};
