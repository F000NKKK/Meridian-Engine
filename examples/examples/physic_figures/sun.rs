//! The sun: its direction, distance and visible disc.

use meridian_sdk::{GraphicsSession, Material, Motor3, Renderable3d, Vec3, icosphere_mesh_source};

/// The one directional light's travel direction — shared by the light
/// itself and the visible "sun" sphere ([`sun_renderable`]) placed
/// opposite it in the sky, so it's never ambiguous where the shadows'
/// light is coming from.
pub(crate) const SUN_DIRECTION: Vec3 = Vec3::new(-0.4, -1.0, -0.3);

/// How far above/behind the scene the visible sun sphere sits — far
/// enough that its own shadow (it's a real renderable, drawn like
/// anything else) never reaches the play area, close enough (with
/// [`SUN_VISUAL_RADIUS`]) to read as an unmistakable bright disc rather
/// than a barely-visible dot once the camera pans toward it.
const SUN_DISTANCE: f32 = 35.0;

const SUN_VISUAL_RADIUS: f32 = 6.0;

/// A small, always-unlit-and-glowing sphere placed opposite
/// [`SUN_DIRECTION`] at [`SUN_DISTANCE`] — makes the scene's one
/// directional light have an obvious, visible source instead of coming
/// from nowhere. Purely decorative (its own mesh/material, no physics
/// body, not part of [`SceneEntity`](crate::SceneEntity)) — it happens to fall inside the
/// shadow camera's fixed volume like everything else, but at that
/// distance and size it never occludes anything the play area's shadow
/// would need.
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
    // The real light's direction is mostly straight down (`SUN_DIRECTION.y
    // = -1.0` dominates), which would put a physically-exact sun almost
    // directly overhead — technically correct, but the free-fly camera
    // starts at pitch `0` (looking at the horizon), so a new viewer
    // would have to already know to pitch steeply upward to ever find
    // it. This flattens the *visual* placement's elevation (still in
    // the light's general horizontal direction, just not as steep) so
    // it's findable by panning around near the horizon instead — a
    // deliberate, disclosed fudge between "physically exact" and
    // "a first-time viewer can actually find it."
    let visual_direction = Vec3::new(SUN_DIRECTION.x, -0.5, SUN_DIRECTION.z).normalize();
    let position = visual_direction * -SUN_DISTANCE;
    Renderable3d {
        mesh,
        material,
        frame: Motor3::translation(position),
        billboard: false,
        // Sitting along the light's own incoming direction, this would
        // otherwise occlude the light for the whole scene below it —
        // see `Renderable3d::casts_shadow`'s own doc comment.
        casts_shadow: false,
        palette: None,
        morph_weights: Vec::new(),
    }
}
