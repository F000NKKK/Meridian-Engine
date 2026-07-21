//! Scene vocabulary and per-frame extraction — see docs/graphics-design.md
//! for the accepted design this module implements: one spatial model
//! (`Motor3`) shared by [`Scene3D`] (the world) and [`Scene2D`] (a
//! layer-sorted plane; UI is a role played by either, not a third kind).
//!
//! Everything a renderable references is a typed `resource-core` handle
//! ([`MeshHandle`], [`TextureHandle`], [`MaterialHandle`]) per
//! [ADR 002](../../../docs/adr/002-handle-based-resources.md) — never a
//! driver object or a raw index. [`Mesh`] and [`Material`] here are the
//! CPU-side, driver-free description of what those handles name; the
//! future submission bridge is the only place that resolves a handle to
//! an actual GPU resource.

use meridian_ecs_core::{Component, Transform, World};
use meridian_gac_core::{Aabb, Motor3};
use meridian_resource_core::ResourceId;

use crate::{Frustum, MeshHandle, TextureHandle};

/// Typed identity for a material — see the module doc's handle note.
pub struct MaterialMarker;
pub type MaterialHandle = ResourceId<MaterialMarker>;

/// How a [`Material`] composites with what's already drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BlendMode {
    #[default]
    Opaque,
    AlphaBlend,
}

/// A surface's shading inputs. Pure data — mapping this onto an actual
/// GPU pipeline is the submission bridge's job, not this type's.
#[derive(Debug, Clone, Copy, Default)]
pub struct Material {
    /// Sampled and multiplied with `base_color_factor` when present;
    /// `base_color_factor` alone otherwise.
    pub albedo: Option<TextureHandle>,
    /// RGBA multiplier — the material's color when unlit or textureless.
    pub base_color_factor: [f32; 4],
    /// Skips lighting entirely — the world-space UI / unshaded case (see
    /// the module doc's "UI is a role" note).
    pub unlit: bool,
    /// Draws after everything else, ignoring depth — the other half of
    /// world-space UI (a cockpit readout that must never be occluded).
    pub always_on_top: bool,
    pub blend: BlendMode,
}

/// A mesh's identity plus the CPU-side info culling needs. The GPU
/// buffers behind [`handle`](Self::handle) belong to the submission
/// bridge; this type never touches them.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub handle: MeshHandle,
    pub vertex_count: u32,
    /// Zero for a non-indexed mesh drawn directly from `vertex_count`.
    pub index_count: u32,
    /// Local-space bounds, transformed per-instance for culling — see
    /// [`world_space_bounds`].
    pub bounds: Aabb,
}

/// A light contributing to a [`Scene3D`]. First model is Blinn-Phong
/// forward shading; a PBR upgrade changes the shader, not this
/// vocabulary (see docs/graphics-design.md).
#[derive(Debug, Clone, Copy)]
pub enum Light {
    /// Uniform light from a fixed direction, no falloff (the sun).
    Directional {
        direction: Motor3,
        color: [f32; 3],
        intensity: f32,
    },
    /// Falls off with distance from `position`, out to `range`.
    Point {
        position: Motor3,
        color: [f32; 3],
        intensity: f32,
        range: f32,
    },
}

/// One 3D-scene instance: a mesh drawn with a material at a world frame.
#[derive(Debug, Clone)]
pub struct Renderable3D {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub frame: Motor3,
    /// Re-oriented to face the camera at extraction time — the common
    /// case for a world-space UI billboard or an impostor sprite.
    pub billboard: bool,
}

/// Attaches a [`Renderable3D`] (minus its frame, which comes from the
/// entity's own [`Transform`]) to an ECS entity — [`extract_scene3d`]'s
/// input component.
#[derive(Debug, Clone, Copy)]
pub struct MeshRenderer {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub billboard: bool,
}

impl Component for MeshRenderer {}

/// A textured quad in a [`Scene2D`]. `frame`'s translation supplies the
/// quad's `x`/`y`; the remaining `Motor3` degrees of freedom (z, the
/// rotation axes other than the plane normal) are simply unused — see
/// the module doc's "one spatial model" note.
#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub texture: TextureHandle,
    /// Texture-space sub-rectangle (`x, y, width, height`); `None` draws
    /// the whole texture.
    pub source_rect: Option<[f32; 4]>,
    /// Quad size in the scene's units (world units for gameplay, logical
    /// pixels for screen-space UI — the camera decides which).
    pub size: [f32; 2],
    pub tint: [f32; 4],
    pub frame: Motor3,
    /// Painter's-order sort key: lower draws first (further back).
    pub layer: i32,
}

/// An orthographic camera for [`Scene2D`] — no perspective, no frustum,
/// just the visible rectangle. [`from_surface_size`](Self::from_surface_size)
/// is the screen-space-UI camera (pixel units, `y` down, origin
/// top-left); [`from_world_extent`](Self::from_world_extent) is the
/// world-space-2D-gameplay camera (centered, `y` up).
#[derive(Debug, Clone, Copy, Default)]
pub struct Camera2D {
    /// `[left, right, bottom, top]`.
    pub extents: [f32; 4],
}

impl Camera2D {
    pub fn from_world_extent(width: f32, height: f32) -> Self {
        Self {
            extents: [-width / 2.0, width / 2.0, -height / 2.0, height / 2.0],
        }
    }

    pub fn from_surface_size(width: f32, height: f32) -> Self {
        Self {
            extents: [0.0, width, height, 0.0],
        }
    }
}

/// The world: a camera, its visible [`Renderable3D`]s, and the lights
/// shading them.
#[derive(Debug, Clone, Default)]
pub struct Scene3D {
    pub camera: crate::Camera,
    pub renderables: Vec<Renderable3D>,
    pub lights: Vec<Light>,
}

/// A layer-sorted plane: an orthographic camera and its [`Sprite`]s.
#[derive(Debug, Clone, Default)]
pub struct Scene2D {
    pub camera: Camera2D,
    pub sprites: Vec<Sprite>,
}

/// One entry in a [`FrameScene`]'s view list — a `Scene3D` or a
/// `Scene2D`, drawn in list order (later views draw over earlier ones;
/// see the module doc's UI-overlay composition).
#[derive(Debug, Clone)]
pub enum View {
    Three(Scene3D),
    Two(Scene2D),
}

/// What one frame's extraction produces: an ordered list of views ready
/// to submit. Rebuilt every frame from ECS state (ADR 004's
/// data-oriented stance) — never a retained scene graph.
#[derive(Debug, Clone, Default)]
pub struct FrameScene {
    pub views: Vec<View>,
}

impl FrameScene {
    pub fn new(views: Vec<View>) -> Self {
        Self { views }
    }

    /// The common single-3D-view case (no UI overlay).
    pub fn single_3d(scene: Scene3D) -> Self {
        Self {
            views: vec![View::Three(scene)],
        }
    }
}

/// Extracts a [`Scene3D`] from `world`: every entity with both a
/// [`Transform`] and a [`MeshRenderer`] becomes a [`Renderable3D`] at
/// its transform's frame. `ecs-core` never learns about any of this —
/// the join happens here, in `graphics-core`, per dependency-rules
/// rule 6 (ECS stays generic and data-oriented).
///
/// Lights aren't extracted yet (no light component exists); callers add
/// them to the returned scene directly until one does.
pub fn extract_scene3d(world: &World, camera: crate::Camera) -> Scene3D {
    let renderables = world
        .query::<MeshRenderer>()
        .filter_map(|(entity, renderer)| {
            let transform = world.get::<Transform>(entity)?;
            Some(Renderable3D {
                mesh: renderer.mesh,
                material: renderer.material,
                frame: transform.motor,
                billboard: renderer.billboard,
            })
        })
        .collect();
    Scene3D {
        camera,
        renderables,
        lights: Vec::new(),
    }
}

/// `bounds` (a mesh's local-space AABB) transformed by `frame` into
/// world space: every corner is mapped through `frame` and the result is
/// the enclosing box of those eight points. Conservative like every
/// AABB-under-rotation approximation (the box can grow, never shrink),
/// which is exactly what a culling test needs — a false "visible" costs
/// a wasted draw, a false "culled" is a visible bug.
pub fn world_space_bounds(bounds: &Aabb, frame: &Motor3) -> Aabb {
    use meridian_gac_core::Vec3;

    let (min, max) = (bounds.min, bounds.max);
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(max.x, max.y, max.z),
    ]
    .map(|corner| frame.transform_point(corner));

    let mut world_min = corners[0];
    let mut world_max = corners[0];
    for corner in &corners[1..] {
        world_min = Vec3::new(
            world_min.x.min(corner.x),
            world_min.y.min(corner.y),
            world_min.z.min(corner.z),
        );
        world_max = Vec3::new(
            world_max.x.max(corner.x),
            world_max.y.max(corner.y),
            world_max.z.max(corner.z),
        );
    }
    Aabb {
        min: world_min,
        max: world_max,
    }
}

/// Frustum-culls `scene`'s renderables against `frustum`, using
/// `mesh_bounds` to look up each renderable's local-space bounds (a
/// caller-supplied lookup rather than a field on [`Renderable3D`] itself
/// — the scene only names a [`MeshHandle`]; resolving it to a [`Mesh`]'s
/// bounds is whatever registry owns loaded meshes, not this function's
/// business). A renderable whose mesh isn't found is kept (fail open:
/// an unresolvable bounds check must not silently make something
/// disappear).
pub fn cull_scene3d(
    scene: &Scene3D,
    frustum: &Frustum,
    mesh_bounds: impl Fn(MeshHandle) -> Option<Aabb>,
) -> Vec<Renderable3D> {
    scene
        .renderables
        .iter()
        .filter(|renderable| match mesh_bounds(renderable.mesh) {
            Some(local_bounds) => {
                let bounds = world_space_bounds(&local_bounds, &renderable.frame);
                frustum.intersects(&bounds)
            }
            None => true,
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_gac_core::{Projection, Vec3};

    fn forward_frustum() -> Frustum {
        let camera = crate::Camera {
            frame: Motor3::identity(),
            projection: Projection::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0),
        };
        Frustum::from_view_projection(camera.view_projection_matrix())
    }

    #[test]
    fn world_space_bounds_translates_local_box_to_world() {
        let local = Aabb {
            min: Vec3::new(-1.0, -1.0, -1.0),
            max: Vec3::new(1.0, 1.0, 1.0),
        };
        let frame = Motor3::translation(Vec3::new(10.0, 0.0, 0.0));
        let world = world_space_bounds(&local, &frame);
        assert_eq!(world.min, Vec3::new(9.0, -1.0, -1.0));
        assert_eq!(world.max, Vec3::new(11.0, 1.0, 1.0));
    }

    #[test]
    fn extract_scene3d_joins_transform_and_mesh_renderer() {
        let mut world = World::new();

        let visible = world.spawn();
        world.insert(
            visible,
            Transform {
                motor: Motor3::translation(Vec3::new(5.0, 0.0, 0.0)),
            },
        );
        world.insert(
            visible,
            MeshRenderer {
                mesh: MeshHandle::new(meridian_memory_core::Handle::default()),
                material: MaterialHandle::new(meridian_memory_core::Handle::default()),
                billboard: false,
            },
        );

        // An entity with a Transform but no MeshRenderer must not appear.
        let bystander = world.spawn();
        world.insert(bystander, Transform::default());

        let camera = crate::Camera::default();
        let scene = extract_scene3d(&world, camera);

        assert_eq!(scene.renderables.len(), 1);
        assert_eq!(
            scene.renderables[0].frame,
            Motor3::translation(Vec3::new(5.0, 0.0, 0.0))
        );
    }

    #[test]
    fn cull_scene3d_drops_renderables_outside_the_frustum() {
        let ahead = Renderable3D {
            mesh: MeshHandle::new(meridian_memory_core::Handle::default()),
            material: MaterialHandle::new(meridian_memory_core::Handle::default()),
            frame: Motor3::translation(Vec3::new(10.0, 0.0, 0.0)),
            billboard: false,
        };
        let behind = Renderable3D {
            mesh: MeshHandle::new(meridian_memory_core::Handle::default()),
            material: MaterialHandle::new(meridian_memory_core::Handle::default()),
            frame: Motor3::translation(Vec3::new(-10.0, 0.0, 0.0)),
            billboard: false,
        };
        let scene = Scene3D {
            camera: crate::Camera::default(),
            renderables: vec![ahead.clone(), behind],
            lights: Vec::new(),
        };

        let unit_cube = Aabb {
            min: Vec3::new(-0.5, -0.5, -0.5),
            max: Vec3::new(0.5, 0.5, 0.5),
        };
        let visible = cull_scene3d(&scene, &forward_frustum(), |_| Some(unit_cube));

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].frame, ahead.frame);
    }

    #[test]
    fn cull_scene3d_keeps_renderables_with_unresolvable_bounds() {
        let renderable = Renderable3D {
            mesh: MeshHandle::new(meridian_memory_core::Handle::default()),
            material: MaterialHandle::new(meridian_memory_core::Handle::default()),
            frame: Motor3::translation(Vec3::new(-10.0, 0.0, 0.0)),
            billboard: false,
        };
        let scene = Scene3D {
            camera: crate::Camera::default(),
            renderables: vec![renderable],
            lights: Vec::new(),
        };

        let visible = cull_scene3d(&scene, &forward_frustum(), |_| None);
        assert_eq!(visible.len(), 1, "unresolvable bounds must fail open");
    }
}
