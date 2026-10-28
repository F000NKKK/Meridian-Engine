//! Extra scene pieces that show the material states: translucent glass and
//! an always-on-top marker.

use meridian_sdk::{
    BlendMode, GraphicsSession, Material, Motor3, Renderable3d, Vec3, cube_mesh_source,
    icosphere_mesh_source,
};

use crate::sun_renderable;

/// How many posts stand in the ring.
const POST_COUNT: u16 = 16;

/// The sun, a translucent glass sphere around the orbit's centre (lit,
/// drawn after the shapes it surrounds so they show through), a polished
/// and a matte sphere side by side, and a small marker sunk below the floor
/// that `always_on_top` draws over it.
pub(crate) fn scene_extras(base: &mut GraphicsSession) -> Vec<Renderable3d> {
    let mut extras = post_ring(base);
    extras.extend([
        sun_renderable(base),
        sphere(
            base,
            Material {
                base_color_factor: [0.5, 0.75, 1.0, 0.25],
                blend: BlendMode::AlphaBlend,
                ..Default::default()
            },
            (1.4, Vec3::new(0.0, 1.5, 0.0)),
        ),
        sphere(
            base,
            Material {
                base_color_factor: [0.8, 0.8, 0.85, 1.0],
                roughness: 0.1,
                ..Default::default()
            },
            (0.6, Vec3::new(-2.2, 0.6, 1.0)),
        ),
        sphere(
            base,
            Material {
                base_color_factor: [0.8, 0.8, 0.85, 1.0],
                roughness: 1.0,
                ..Default::default()
            },
            (0.6, Vec3::new(2.2, 0.6, 1.0)),
        ),
        sphere(
            base,
            Material {
                base_color_factor: [1.0, 0.3, 0.3, 1.0],
                unlit: true,
                always_on_top: true,
                ..Default::default()
            },
            (0.3, Vec3::new(0.0, -0.3, 2.0)),
        ),
    ]);
    extras
}

/// [`POST_COUNT`] small posts in a ring around the orbit: one mesh and one
/// material, so the renderer draws them all in a single instanced call.
fn post_ring(base: &mut GraphicsSession) -> Vec<Renderable3d> {
    let mesh = base
        .render
        .meshes
        .register(cube_mesh_source(0.12))
        .expect("post mesh must be valid");
    let material = base.render.materials.register(Material {
        base_color_factor: [0.55, 0.5, 0.45, 1.0],
        ..Default::default()
    });
    (0..POST_COUNT)
        .map(|index| {
            let angle = f32::from(index) / f32::from(POST_COUNT) * std::f32::consts::TAU;
            Renderable3d {
                mesh,
                material,
                frame: Motor3::translation(Vec3::new(angle.cos() * 5.0, 0.12, angle.sin() * 5.0)),
                scale: Vec3::ONE,
                billboard: false,
                casts_shadow: true,
                palette: None,
                morph_weights: Vec::new(),
            }
        })
        .collect()
}

/// An icosphere of `(radius, position)` with `material`; it casts no shadow.
fn sphere(
    base: &mut GraphicsSession,
    material: Material,
    (radius, position): (f32, Vec3),
) -> Renderable3d {
    let mesh = base
        .render
        .meshes
        .register(icosphere_mesh_source(2, radius))
        .expect("sphere mesh must be valid");
    Renderable3d {
        mesh,
        material: base.render.materials.register(material),
        frame: Motor3::translation(position),
        scale: Vec3::ONE,
        billboard: false,
        casts_shadow: false,
        palette: None,
        morph_weights: Vec::new(),
    }
}
