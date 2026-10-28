//! The sun: its direction, distance and visible disc.

use meridian_sdk::{GraphicsSession, Material, Motor3, Renderable3d, Vec3, icosphere_mesh_source};

/// The one directional light's travel direction and the visible "sun"
/// sphere placed opposite it — see `physic_figures`' identical constants
/// for why the *visual* placement's elevation is deliberately flattened
/// relative to the physically-exact direction (findable by panning near
/// the horizon, not only by pitching steeply upward).
pub(crate) const SUN_DIRECTION: Vec3 = Vec3::new(-0.4, -1.0, -0.3);

const SUN_DISTANCE: f32 = 35.0;

const SUN_VISUAL_RADIUS: f32 = 6.0;

/// A small, always-unlit-and-glowing sphere placed opposite
/// `SUN_DIRECTION` — see `physic_figures::sun_renderable`'s identical
/// doc comment for why the visual elevation is flattened relative to
/// the physically-exact light direction.
pub(crate) fn sun_renderable(base: &mut GraphicsSession) -> Renderable3d {
    let mesh = base
        .render
        .meshes
        .register(icosphere_mesh_source(1, SUN_VISUAL_RADIUS))
        .expect("sun mesh must be valid");
    let material = base.render.materials.register(Material {
        base_color_factor: [1.0, 0.95, 0.8, 1.0],
        unlit: true,
        emissive: [1.0, 0.95, 0.8],
        ..Default::default()
    });
    let visual_direction = Vec3::new(SUN_DIRECTION.x, -0.5, SUN_DIRECTION.z).normalize();
    let position = visual_direction * -SUN_DISTANCE;
    Renderable3d {
        mesh,
        material,
        frame: Motor3::translation(position),
        scale: Vec3::ONE,
        billboard: false,
        // Sitting along the light's own incoming direction, this would
        // otherwise occlude the light for the whole scene below it —
        // see `Renderable3d::casts_shadow`'s own doc comment.
        casts_shadow: false,
        palette: None,
        morph_weights: Vec::new(),
    }
}
