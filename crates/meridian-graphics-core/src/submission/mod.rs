//! The submission bridge: turns a culled [`Scene3d`](crate::scene::Scene3d) into real GPU draws
//! through `graphics-driver` — step 2 of docs/graphics-design.md's scene
//! vocabulary implementation order (texture-sampled materials), plus
//! step 3 (Blinn-Phong lighting). This is the one place in
//! `graphics-core` allowed to touch `graphics-driver` objects; every
//! other type in this crate stays driver-free. [`crate::bloom`] builds
//! on top of the emissive channel this module bakes per-vertex.
//!
//! **Mesh/material/texture identity is handle-based, never a raw index
//! or a driver object** ([`MeshRegistry`]/[`MaterialRegistry`]/
//! [`TextureRegistry`] wrap `memory-core::ResourcePool`, resolved by
//! [`MeshHandle`](crate::MeshHandle)/[`MaterialHandle`](crate::MaterialHandle)/[`TextureHandle`](crate::TextureHandle) — see ADR 002).
//!
//! **Four pipelines, chosen per renderable at bake time:** textured
//! vs. flat-color (whether `Material::albedo` resolves in the
//! [`TextureRegistry`]) crossed with lit vs. `Material::unlit`.
//!
//! Lit pipelines run Blinn-Phong forward shading against
//! `Scene3d::lights` + `Scene3d::ambient` — a fixed-shininess,
//! no-shadows approximation (real material roughness/specular control
//! and shadow mapping are future work, not this step); unlit pipelines
//! skip lighting entirely, for world-space UI and anything meant to
//! read at a constant brightness. `Material::emissive` is added on top
//! in both cases and baked into every vertex regardless of family — see
//! `bake_draw_buffers` — so a mesh can glow (feed a bloom pass)
//! whether or not it's lit.
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
mod view_uniforms;

pub use draw_buffers::DrawBuffers;
pub(crate) use draw_buffers::bake_draw_buffers;
pub(crate) use draw_kind::DrawKind;
pub(crate) use draw_request::DrawRequest;
pub use prepared_scene::PreparedScene;
pub use registries::*;
pub use scene_assets::SceneAssets;
pub use scene_renderer::{DEFAULT_SHADOW_MAP_SIZE, SceneRenderer};
pub use shaders::*;
pub use shadows::*;
pub use submit_scene3d::submit_scene3d;
pub(crate) use uniform_bytes::{lit_uniform_bytes, mat4_to_bytes};
pub(crate) use vertex_layout::{
    colored_vertex_layout, emissive_from_colored_layout, emissive_from_textured_layout,
    shadow_from_colored_layout, shadow_from_textured_layout, textured_vertex_layout,
};
pub use view_uniforms::ViewUniforms;
