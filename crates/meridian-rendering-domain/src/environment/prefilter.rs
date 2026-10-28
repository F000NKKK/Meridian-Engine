//! GGX prefiltering of an environment for one roughness.

use core::f32::consts::{PI, TAU};

use super::EnvironmentLevel;
use super::direction::uv_to_direction;

/// Importance samples per output texel.
const SAMPLES: u32 = 64;

/// The `index`-th point of the `count`-point Hammersley set in the unit square.
fn hammersley(index: u32, count: u32) -> [f32; 2] {
    [
        (index as f32 + 0.5) / count as f32,
        index.reverse_bits() as f32 * 2.328_306_4e-10,
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalized(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt().max(1e-12);
    v.map(|c| c / length)
}

/// Two unit vectors completing `normal` to an orthonormal frame.
fn frame(normal: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let up = if normal[1].abs() < 0.999 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let tangent = normalized(cross(up, normal));
    (tangent, cross(normal, tangent))
}

/// A GGX-distributed half vector around `normal` for sample point `xi`, with
/// the distribution's value at it.
fn ggx_half_vector(xi: [f32; 2], alpha: f32, normal: [f32; 3]) -> ([f32; 3], f32) {
    let a2 = alpha * alpha;
    let phi = TAU * xi[0];
    let cos_theta = ((1.0 - xi[1]) / (1.0 + (a2 - 1.0) * xi[1])).sqrt();
    let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
    let (tangent, bitangent) = frame(normal);
    let local = [sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta];
    let half =
        [0, 1, 2].map(|c| tangent[c] * local[0] + bitangent[c] * local[1] + normal[c] * local[2]);
    let denominator = cos_theta * cos_theta * (a2 - 1.0) + 1.0;
    (half, a2 / (PI * denominator * denominator))
}

/// Blends the two pyramid levels around `lod` at direction `direction`.
fn sample_pyramid(pyramid: &[EnvironmentLevel], lod: f32, direction: [f32; 3]) -> [f32; 3] {
    let top = (pyramid.len() - 1) as f32;
    let lod = lod.clamp(0.0, top);
    let (low, high) = (lod.floor() as usize, lod.ceil() as usize);
    let blend = lod - lod.floor();
    let a = pyramid[low].sample_direction(direction);
    let b = pyramid[high].sample_direction(direction);
    [0, 1, 2].map(|c| a[c] + (b[c] - a[c]) * blend)
}

/// The environment `pyramid` (sharpest level first) as a surface of
/// `roughness` reflects it, as a `width` x `height` image. Each texel
/// integrates GGX-weighted samples around its direction, read from the pyramid
/// level whose texels match each sample's solid angle, which keeps the result
/// smooth with few samples.
#[must_use]
pub(super) fn prefilter(
    pyramid: &[EnvironmentLevel],
    roughness: f32,
    width: u32,
    height: u32,
) -> EnvironmentLevel {
    let alpha = roughness * roughness;
    let source = &pyramid[0];
    let texel_solid_angle = 4.0 * PI / (source.width as f32 * source.height as f32);
    let mut texels = Vec::with_capacity((width * height) as usize);
    for row in 0..height {
        for column in 0..width {
            let normal = uv_to_direction(
                (column as f32 + 0.5) / width as f32,
                (row as f32 + 0.5) / height as f32,
            );
            let mut sum = [0.0f32; 3];
            let mut total = 0.0f32;
            for index in 0..SAMPLES {
                let (half, density) = ggx_half_vector(hammersley(index, SAMPLES), alpha, normal);
                let n_dot_h = dot(normal, half);
                let light = [0, 1, 2].map(|c| 2.0 * n_dot_h * half[c] - normal[c]);
                let n_dot_l = dot(normal, light);
                if n_dot_l <= 0.0 {
                    continue;
                }
                let sample_solid_angle = 1.0 / (SAMPLES as f32 * density * 0.25 + 1e-4);
                let lod = 0.5 * (sample_solid_angle / texel_solid_angle).log2() + 1.0;
                let radiance = sample_pyramid(pyramid, lod, light);
                for (channel, value) in sum.iter_mut().zip(radiance) {
                    *channel += value * n_dot_l;
                }
                total += n_dot_l;
            }
            texels.push(sum.map(|c| c / total.max(1e-6)));
        }
    }
    EnvironmentLevel {
        width,
        height,
        texels,
    }
}
