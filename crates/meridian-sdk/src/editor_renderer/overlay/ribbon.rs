//! Lines as thin camera-facing quads: the renderer has no line primitive.

use meridian_gac::Vec3;
use meridian_rendering_domain::MeshSource;

/// One segment to draw, `width` pixels thick.
#[derive(Debug, Clone, Copy)]
pub(super) struct Line {
    pub(super) from: Vec3,
    pub(super) to: Vec3,
    pub(super) width: f32,
}

/// The mesh of `lines` seen from `eye`: each is a quad facing the eye,
/// `world_per_pixel(distance)` world units per pixel at its distance, so a
/// line keeps its width on screen. `None` for no (visible) line.
pub(super) fn ribbon_mesh(
    lines: &[Line],
    eye: Vec3,
    world_per_pixel: impl Fn(f32) -> f32,
) -> Option<MeshSource> {
    let mut mesh = MeshSource {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        indices: Vec::new(),
        skin: None,
        morph: None,
    };
    for line in lines {
        push_quad(&mut mesh, line, eye, &world_per_pixel);
    }
    (!mesh.indices.is_empty()).then_some(mesh)
}

fn push_quad(mesh: &mut MeshSource, line: &Line, eye: Vec3, world_per_pixel: &impl Fn(f32) -> f32) {
    let middle = Vec3::new(
        (line.from.x + line.to.x) * 0.5,
        (line.from.y + line.to.y) * 0.5,
        (line.from.z + line.to.z) * 0.5,
    );
    let view = middle - eye;
    let along = line.to - line.from;
    let side = along.cross(view);
    if side.length_squared() < 1e-12 {
        return;
    }
    let half = side.normalize() * (world_per_pixel(view.length()) * line.width * 0.5);
    let facing = -view.normalize();
    let first = u32::try_from(mesh.positions.len()).unwrap_or(u32::MAX);
    for corner in [
        line.from - half,
        line.from + half,
        line.to + half,
        line.to - half,
    ] {
        mesh.positions.push([corner.x, corner.y, corner.z]);
        mesh.normals.push([facing.x, facing.y, facing.z]);
        mesh.uvs.push([0.0, 0.0]);
    }
    // Both windings, so the quad shows whichever way it is culled.
    for [a, b, c] in [[0, 1, 2], [0, 2, 3], [0, 2, 1], [0, 3, 2]] {
        mesh.indices.extend([first + a, first + b, first + c]);
    }
}
