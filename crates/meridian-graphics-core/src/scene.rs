//! Scene vocabulary: renderable types, lights, cameras, and per-frame scene extraction.
//!
//! This module provides the scene types described in `docs/graphics-design.md`:
//! - Scene resources (`Mesh`, `Material`) identified by `resource-core` handles
//! - Renderables (`Renderable3D`, `Sprite`) referenced by the scene
//! - `Scene3D`/`Scene2D` container types
//! - `FrameScene` as the per-frame extraction output

use meridian_gac_core::{Aabb, Motor3};
use meridian_resource_core::ResourceId;

use super::{MeshHandle, TextureHandle};

/// Blend mode for material rendering. GPU-specific factor mapping is handled
/// by the GPU bridge, not in the core types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BlendMode {
    /// Standard opaque rendering.
    #[default]
    Opaque,
    /// Alpha blending.
    AlphaBlend,
}

/// A material's shading inputs. Pure data; the submission bridge maps these
/// onto actual GPU pipelines.
#[derive(Debug, Clone, Copy, Default)]
pub struct Material {
    /// Optional albedo texture. When `None`, only `base_color_factor` is used.
    pub albedo: Option<TextureHandle>,
    /// Base color multiplier when no texture or as a fallback.
    pub base_color_factor: [f32; 4], // RGBA
    /// When true, the material is rendered unlit (no lighting calculations).
    pub unlit: bool,
    /// When true, the material is rendered on top of everything else (world-space UI).
    pub always_on_top: bool,
    /// Blend mode for transparency.
    pub blend: BlendMode,
}

/// A GPU mesh: identity + CPU-side layout info for culling. The actual GPU
/// buffers are owned by the submission bridge, not here.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub handle: MeshHandle,
    /// Number of vertices in this mesh.
    pub vertex_count: u32,
    /// Number of indices (if indexed). Zero if the mesh uses `vertex_count` vertices directly.
    pub index_count: u32,
    /// Axis-aligned bounding box for frustum culling.
    pub bounds: Aabb,
}

/// A point light in the scene.
#[derive(Debug, Clone, Copy)]
pub struct Light {
    pub kind: LightKind,
}

/// Light variant types.
#[derive(Debug, Clone, Copy)]
pub enum LightKind {
    /// Directional light at infinity.
    Directional {
        /// Direction in world space (will be normalized).
        direction: Motor3,
        /// Light color in linear space.
        color: [f32; 3],
        /// Intensity multiplier.
        intensity: f32,
    },
    /// Point light at a position.
    Point {
        /// Position in world space.
        position: Motor3,
        /// Light color in linear space.
        color: [f32; 3],
        /// Intensity multiplier.
        intensity: f32,
        /// Maximum distance the light affects.
        range: f32,
    },
}

/// A 3D renderable: mesh + material + transform.
#[derive(Debug, Clone)]
pub struct Renderable3D {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    /// World-space transform.
    pub frame: Motor3,
    /// When true, the mesh is re-oriented to face the camera at extraction time.
    pub billboard: bool,
}

/// A 2D sprite: textured quad.
#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub texture: TextureHandle,
    /// Optional source rectangle in texture space (defaults to full texture).
    pub source_rect: Option<[f32; 4]>, // x, y, width, height
    /// Size in world units (or pixels for UI).
    pub size: [f32; 2], // width, height
    /// Tint color (multiplied with texture).
    pub tint: [f32; 4], // RGBA
    /// World-space transform (constrained to z=0 plane for 2D rendering).
    pub frame: Motor3,
    /// Painter's-order sorting layer (lower = behind).
    pub layer: i32,
}

/// Typed handle for materials.
pub struct MaterialMarker;
pub type MaterialHandle = ResourceId<MaterialMarker>;

/// A 2D camera: orthographic with explicit bounds.
#[derive(Debug, Clone, Copy, Default)]
pub struct Camera2D {
    /// Orthographic extents: left, right, bottom, top.
    pub extents: [f32; 4],
}

impl Camera2D {
    /// Creates a camera for world-space 2D gameplay.
    pub fn from_world_extent(width: f32, height: f32) -> Self {
        Self {
            extents: [-width / 2.0, width / 2.0, -height / 2.0, height / 2.0],
        }
    }

    /// Creates a camera for screen-space UI (pixel units, Y-down).
    pub fn from_surface_size(width: f32, height: f32) -> Self {
        Self {
            // Y-down, origin at top-left.
            extents: [0.0, width, height, 0.0],
        }
    }
}

/// A 3D scene: camera + renderables + lights.
#[derive(Debug, Clone, Default)]
pub struct Scene3D {
    pub camera: super::Camera,
    pub renderables: Vec<Renderable3D>,
    pub lights: Vec<Light>,
}

/// A 2D scene: camera + sprites.
#[derive(Debug, Clone, Default)]
pub struct Scene2D {
    pub camera: Camera2D,
    pub sprites: Vec<Sprite>,
}

/// One view: a scene + camera + target. Used for compositing.
#[derive(Debug, Clone)]
pub enum View {
    Three(Scene3D),
    Two(Scene2D),
}

/// Per-frame extraction output: ordered list of views to render.
/// Frame-scoped plain data, rebuilt every frame.
#[derive(Debug, Clone, Default)]
pub struct FrameScene {
    pub views: Vec<View>,
}

impl FrameScene {
    pub fn new(views: Vec<View>) -> Self {
        Self { views }
    }

    pub fn single_3d(scene: Scene3D) -> Self {
        Self {
            views: vec![View::Three(scene)],
        }
    }
}
