//! The sphere around everything a scene draws.

use meridian_rendering_domain::{MeshRegistry, Scene3d};

/// Centre and radius of a sphere around every vertex of `scene`'s renderables
/// (a skinned mesh at its bind pose); `None` when it draws nothing.
pub(super) fn bounding_sphere(scene: &Scene3d, meshes: &MeshRegistry) -> Option<([f32; 3], f32)> {
    let points: Vec<[f32; 3]> = scene
        .renderables
        .iter()
        .filter_map(|renderable| {
            let source = meshes.get(renderable.mesh)?;
            Some(source.positions.iter().map(move |p| {
                let world = renderable
                    .frame
                    .transform_point(meridian_gac::Vec3::new(p[0], p[1], p[2]));
                [world.x, world.y, world.z]
            }))
        })
        .flatten()
        .collect();
    let (low, high) = points
        .iter()
        .fold(([f32::MAX; 3], [f32::MIN; 3]), |(low, high), point| {
            (
                std::array::from_fn(|axis| low[axis].min(point[axis])),
                std::array::from_fn(|axis| high[axis].max(point[axis])),
            )
        });
    if points.is_empty() {
        return None;
    }
    let center: [f32; 3] = std::array::from_fn(|axis| low[axis].midpoint(high[axis]));
    let half: [f32; 3] = std::array::from_fn(|axis| high[axis] - center[axis]);
    let radius = (half[0] * half[0] + half[1] * half[1] + half[2] * half[2]).sqrt();
    Some((center, radius.max(0.01)))
}
