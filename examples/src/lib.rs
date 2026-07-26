//! Shared rendering scaffolding for the windowed examples
//! (`magic_figures`, `physic_figures`) — mesh builders, texture loading,
//! and the `SceneRenderer`/`BloomPass`/registry bundle both build
//! identically (see [`scene_base::GraphicsBase`]), plus the shared
//! `FlyCamera`. Not part of any published crate (`publish = false` on
//! this package) — this is example-only scaffolding, not a new engine
//! API.

pub mod fly_camera;
pub mod scene_base;

pub use fly_camera::{FlyCamera, look_at_rotor};
pub use scene_base::{
    GraphicsBase, cube_mesh_source, ground_mesh_source, icosphere_mesh_source, load_image_asset,
    pyramid_mesh_source,
};
