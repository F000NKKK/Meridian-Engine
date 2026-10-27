//! Extra scene pieces that show the material states: translucent glass and
//! an always-on-top marker.

use meridian_sdk::{
    BlendMode, GraphicsSession, Material, Motor3, Renderable3d, Vec3, icosphere_mesh_source,
};

use crate::sun_renderable;

/// The sun, a translucent glass sphere around the orbit's centre (lit,
/// drawn after the shapes it surrounds so they show through), a polished
/// and a matte sphere side by side, and a small marker sunk below the floor
/// that `always_on_top` draws over it.
pub(crate) fn scene_extras(base: &mut GraphicsSession) -> Vec<Renderable3d> {
    vec![
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
                shininess: 200.0,
                specular: 1.0,
                ..Default::default()
            },
            (0.6, Vec3::new(-2.2, 0.6, 1.0)),
        ),
        sphere(
            base,
            Material {
                base_color_factor: [0.8, 0.8, 0.85, 1.0],
                specular: 0.0,
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
    ]
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
        billboard: false,
        casts_shadow: false,
    }
}
