//! Diffuse irradiance as nine spherical-harmonic coefficients.

use core::f32::consts::{PI, TAU};

use super::EnvironmentLevel;
use super::direction::uv_to_direction;

/// Real spherical-harmonic basis, bands 0 to 2, at unit direction `d`.
#[must_use]
pub(super) fn basis(d: [f32; 3]) -> [f32; 9] {
    let [x, y, z] = d;
    [
        0.282_095,
        0.488_603 * y,
        0.488_603 * z,
        0.488_603 * x,
        1.092_548 * x * y,
        1.092_548 * y * z,
        0.315_392 * (3.0 * z * z - 1.0),
        1.092_548 * x * z,
        0.546_274 * (x * x - y * y),
    ]
}

/// Per-band factor turning radiance coefficients into `irradiance / pi`
/// (Ramamoorthi and Hanrahan's cosine-lobe convolution, divided by `pi`).
const BAND_FACTOR: [f32; 9] = [
    1.0,
    2.0 / 3.0,
    2.0 / 3.0,
    2.0 / 3.0,
    0.25,
    0.25,
    0.25,
    0.25,
    0.25,
];

/// Projects `level` onto the nine basis functions, weighting each texel by its
/// solid angle, and applies the cosine-lobe convolution: evaluating the result
/// with [`evaluate`] gives the cosine-weighted mean radiance a surface facing
/// `n` receives, which is what a diffuse surface of albedo 1 reflects.
#[must_use]
pub(super) fn irradiance_coefficients(level: &EnvironmentLevel) -> [[f32; 3]; 9] {
    let mut sum = [[0.0f32; 3]; 9];
    let texel_area = TAU * PI / (level.width as f32 * level.height as f32);
    for row in 0..level.height {
        let v = (row as f32 + 0.5) / level.height as f32;
        let solid_angle = texel_area * ((0.5 - v) * PI).cos();
        for column in 0..level.width {
            let u = (column as f32 + 0.5) / level.width as f32;
            let radiance = level.texels[(row * level.width + column) as usize];
            let weights = basis(uv_to_direction(u, v));
            for (coefficient, weight) in sum.iter_mut().zip(weights) {
                for (channel, value) in coefficient.iter_mut().zip(radiance) {
                    *channel += value * weight * solid_angle;
                }
            }
        }
    }
    for (coefficient, factor) in sum.iter_mut().zip(BAND_FACTOR) {
        *coefficient = coefficient.map(|c| c * factor);
    }
    sum
}

/// The irradiance (divided by `pi`) the coefficients give a surface facing unit
/// direction `normal`. The environment shader evaluates the same sum.
#[must_use]
pub(super) fn evaluate(coefficients: &[[f32; 3]; 9], normal: [f32; 3]) -> [f32; 3] {
    let mut sum = [0.0; 3];
    for (coefficient, weight) in coefficients.iter().zip(basis(normal)) {
        for (channel, value) in sum.iter_mut().zip(coefficient) {
            *channel += value * weight;
        }
    }
    sum
}
