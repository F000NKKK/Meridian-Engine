//! The drawing order of a prepared scene.

use meridian_gac::Vec3;
use meridian_rendering_domain::scene::Scene3d;
use meridian_rendering_domain::{OrderedDraw, order_draws};

use super::DrawBuffers;

/// Indices of `buffers` in the order to draw them, measuring translucent
/// distance from `scene`'s camera.
pub(crate) fn draw_order(buffers: &[DrawBuffers], scene: &Scene3d) -> Vec<usize> {
    let eye = scene.camera.frame.transform_point(Vec3::ZERO);
    let draws: Vec<OrderedDraw> = buffers
        .iter()
        .map(|buffer| OrderedDraw {
            state: buffer.state,
            distance_squared: (buffer.origin - eye).length_squared(),
        })
        .collect();
    order_draws(&draws)
}
